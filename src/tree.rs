use im::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

const JOIN_LEN: usize = 511;
const SPLIT_LEN: usize = 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeError {
    UnknownId,
    UnexpectedNodeType,
    DuplicateId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueType {
    String,
    Character,
    True,
    False,
    Null,
    Object,
    Array,
    ArrayEntry,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct NodeId(usize);

/// This struct is left public for others who would like to build their own CRDT library or have a
/// custom setup of some kind. Most crudite users will not need to use this.
///
/// A JSON-compatible document where each character and value in the document has a unique ID, and
/// deletions in arrays and strings maintain tombstones for ordering future insertions. All methods
/// on this tree should be `O(log n)` or better unless otherwise noted. The tree also internally
/// uses persistent data structures, so cloning should be a very fast and efficient operation.
///
/// Sequences like arrays and strings in `Tree` are represented by a persistent double linked list
/// of segments. This is sorta like just the leaves of a rope connected by a doubly linked list.
/// Why not use a rope? Ropes are useful for calculating "what character is at position n" very
/// efficiently. However, it's tricky to make ropes work with random access via IDs, and there is
/// overhead for calculating the rope. We opt instead to make indexed access `O(n)` and ID-based
/// access `O(1)`.
#[derive(Clone, Debug)]
pub struct Tree<Id: Hash + Clone + Eq + Debug> {
    /// Number to use for the next node that is created.
    next_node: NodeId,

    /// Id of the root object of the tree
    root: Id,

    /// Maps external IDs to their position in the tree. In the case of Segments of a sequence,
    /// futher disambiguation may be necessary to find the exact character this represents within
    /// the string.
    id_to_node: HashMap<Id, NodeId>,

    /// Maps node ids to node data.
    nodes: HashMap<NodeId, Node<Id>>,
}

#[derive(Clone, Debug)]
struct Node<Id: Hash + Clone + Eq + Debug> {
    data: NodeData<Id>,
    parent: Option<NodeId>,
}

#[derive(Clone, Debug)]
enum NodeData<Id: Hash + Clone + Eq + Debug> {
    // TODO once string is implemented, copy implementation for `Array`?
    True {
        id: Id,
    },
    False {
        id: Id,
    },
    Null {
        id: Id,
    },
    Object {
        items: HashMap<String, NodeId>,
        id: Id,
    },
    /// Represents a JSON string value.
    String {
        /// The first `TextSegment` in the string value. May be equal to `end` if there is only one
        /// segment.
        start: NodeId,
        /// The last `TextSegment` in the string value. May be equal to `start` if there is only
        /// one segment.
        end: NodeId,
        id: Id,
    },
    /// Represents a range of a JSON string value.
    StringSegment {
        /// Node index of the previous `TextSegment` in this string. If this is the first segment
        /// in the string, refers to the `Text` parent.
        prev: NodeId,
        /// Node index of the next `TextSegment` in this string. If this is the last segment
        /// in the string, refers to the `Text` parent.
        next: NodeId,
        /// String contents of this segment.
        contents: String,
        /// List of ids. If they are a tombstone, the the Option will be None, if they represent a
        /// live character, the Option will show the index of the character.
        ids: Vec<(Id, Option<usize>)>,
    },
}

impl<Id: Hash + Clone + Eq + Debug> Tree<Id> {
    /// This is private since it constructs a tree with no root value; use one of the public
    /// constructors to create the `Tree` instead.
    fn new(root_id: Id) -> Self {
        Tree {
            next_node: NodeId(0),
            id_to_node: HashMap::new(),
            nodes: HashMap::new(),
            root: root_id,
        }
    }

    /// Creates a new `Tree` representing an empty string.
    pub fn new_with_string_root(root_id: Id) -> Self {
        let mut tree = Self::new(root_id.clone());
        tree.construct_string(root_id).unwrap();
        tree
    }

    /// Creates a new `Tree` representing an empty object.
    pub fn new_with_object_root(root_id: Id) -> Self {
        let mut tree = Self::new(root_id.clone());
        tree.construct_object(root_id).unwrap();
        tree
    }

    fn construct_simple(&mut self, id: Id, node_data: NodeData<Id>) -> Result<NodeId, TreeError> {
        if self.id_to_node.contains_key(&id) {
            return Err(TreeError::DuplicateId);
        }
        let node_id = self.next_id();
        self.id_to_node.insert(id, node_id);
        self.nodes.insert(
            node_id,
            Node {
                parent: None,
                data: node_data,
            },
        );
        Ok(node_id)
    }

    /// Constructs a new bool value within the `Tree`. Newly constructed values have no parent or
    /// place in the tree until placed with an `assign` call.
    pub fn construct_bool(&mut self, id: Id, val: bool) -> Result<(), TreeError> {
        self.construct_simple(
            id.clone(),
            if val {
                NodeData::True { id }
            } else {
                NodeData::False { id }
            },
        )
        .map(|_| ())
    }

    /// Constructs a new null value within the `Tree`. Newly constructed values have no parent or
    /// place in the tree until placed with an `assign` call.
    pub fn construct_null(&mut self, id: Id) -> Result<(), TreeError> {
        self.construct_simple(id.clone(), NodeData::Null { id })
            .map(|_| ())
    }

    /// Constructs a new empty object within the `Tree`. Newly constructed values have no parent or
    /// place in the tree until placed with an `assign` call.
    pub fn construct_object(&mut self, id: Id) -> Result<(), TreeError> {
        self.construct_simple(
            id.clone(),
            NodeData::Object {
                items: HashMap::new(),
                id,
            },
        )
        .map(|_| ())
    }

    /// Constructs a new empty string within the `Tree`. Newly constructed values have no parent or
    /// place in the tree until placed with an `assign` call.
    pub fn construct_string(&mut self, id: Id) -> Result<(), TreeError> {
        let segment_id = self.next_id();
        let string_id = self.construct_simple(
            id.clone(),
            NodeData::String {
                id,
                start: segment_id,
                end: segment_id,
            },
        )?;
        self.nodes.insert(
            segment_id,
            Node {
                parent: Some(string_id),
                data: NodeData::StringSegment {
                    contents: "".to_string(),
                    ids: vec![],
                    prev: string_id,
                    next: string_id,
                },
            },
        );
        Ok(())
    }

    fn next_id(&mut self) -> NodeId {
        let res = self.next_node;
        self.next_node.0 += 1;
        res
    }

    /// Deletes a node and all its children. If you want to delete a single segment, try
    /// `delete_segment`.
    fn delete(&mut self, item: NodeId) {
        match self.nodes[&item].data {
            NodeData::True { .. }
            | NodeData::False { .. }
            | NodeData::Null { .. }
            | NodeData::Object { .. }
            | NodeData::String { .. } => { /* do nothing */ }
            _ => panic!("attempted to delete invalid type"),
        }
        let mut queue = vec![item];
        while let Some(item) = queue.pop() {
            let node = match self.nodes.remove(&item) {
                Some(v) => v,
                None => continue,
            };
            match node.data {
                NodeData::True { id } | NodeData::False { id } | NodeData::Null { id } => {
                    // do nothing
                    self.id_to_node.remove(&id).unwrap();
                }
                NodeData::Object { id, items } => {
                    for (_, id) in items {
                        queue.push(id);
                    }
                    self.id_to_node.remove(&id).unwrap();
                }
                NodeData::String { start, id, .. } => {
                    queue.push(start);
                    self.id_to_node.remove(&id).unwrap();
                }
                NodeData::StringSegment { next, ids, .. } => {
                    queue.push(next);
                    for (id, _) in ids {
                        self.id_to_node.remove(&id).unwrap();
                    }
                }
            }
        }
    }

    // TODO right now this is last-write-wins, could modify the object NodeData pretty lightly and
    // get multi value registers which would be sick
    /// Moves `value` to `object[key]`. Since this recursively traverses the children of `object`
    /// it has `O(n log n)` worse case time. If `value` is `None`, the key is deleted.
    pub fn object_assign(&mut self, object: Id, key: String, value: Option<Id>) -> Result<(), TreeError> {
        let object_node_id = *self.id_to_node.get(&object).ok_or(TreeError::UnknownId)?;
        let value_node_id = if let Some(value) = value {
            Some(*self.id_to_node.get(&value).ok_or(TreeError::UnknownId)?)
        } else {
            None
        };
        match &mut self.nodes[&object_node_id].data {
            NodeData::Object { items, id: _ } => {
                let old = if let Some(value_node_id) = value_node_id {
                    items.insert(key, value_node_id)
                } else {
                    items.remove(&key)
                };
                if let Some(old_id) = old {
                    self.delete(old_id);
                }
            }
            _ => return Err(TreeError::UnexpectedNodeType),
        }
        if let Some(value_node_id) = value_node_id {
            self.nodes[&value_node_id].parent = Some(object_node_id);
        }
        Ok(())
    }

    /// Deletes a segment with node id `usize`, returns deleted NodeData and new Tree. Caller is
    /// responsible for updating `id_to_node`, but this takes care of updating `next`, `prev`, or
    /// if necessary, `start` and `end`. If this is the only segment in the list, it will panic.
    fn delete_segment(&mut self, segment: NodeId) -> Node<Id> {
        let segment_data = self.nodes.remove(&segment).expect("segment did not exist");
        let (old_prev, old_next) = match &segment_data.data {
            NodeData::StringSegment { prev, next, .. } => (*prev, *next),
            _ => panic!("delete_segment called on non-segment node"),
        };
        if old_prev == old_next {
            // TODO should this actually panic?
            panic!("attempted to delete only segment in list");
        }
        match &mut self.nodes[&old_prev].data {
            NodeData::StringSegment { next, .. } => *next = old_next,
            NodeData::String { start, .. } => *start = old_next,
            _ => panic!("delete_segment called on non-segment node"),
        }
        match &mut self.nodes[&old_next].data {
            NodeData::StringSegment { prev, .. } => *prev = old_prev,
            NodeData::String { end, .. } => *end = old_prev,
            _ => panic!("delete_segment called on non-segment node"),
        }
        segment_data
    }

    // Inserts a new, empty segment after `append_to`, and returns the usize of the new node.
    fn insert_segment(&mut self, append_to: NodeId) -> NodeId {
        let new_id = self.next_id();
        let (parent, prev, next) = match &mut self.nodes[&append_to] {
            Node {
                parent,
                data: NodeData::StringSegment { prev, next, .. },
            } => {
                let old_next = *next;
                *next = new_id;
                (*parent, append_to, old_next)
            }
            Node {
                parent: _,
                data: NodeData::String { start, .. },
            } => {
                let old_start = *start;
                *start = new_id;
                (Some(append_to), append_to, old_start)
            }
            _ => panic!("insert_segment called on non-segment node"),
        };
        let node = Node {
            parent,
            data: NodeData::StringSegment {
                prev,
                next,
                contents: String::new(),
                ids: Vec::new(),
            },
        };
        self.nodes.insert(new_id, node);
        match &mut self.nodes[&next].data {
            NodeData::StringSegment { prev, .. } => {
                *prev = new_id;
            }
            NodeData::String { end, .. } => {
                *end = new_id;
            }
            _ => panic!("insert_segment called on non-segment node"),
        }
        new_id
    }

    /// If either `segment` or the next node are less than `JOIN_LEN`, and together they are
    /// less than `SPLIT_LEN`, then this function joins them together. In all other cases, it is a
    /// no-op.
    fn consider_join(&mut self, segment: NodeId) {
        let (segment_len, next) = match &self.nodes[&segment].data {
            NodeData::StringSegment { ids, next, .. } => (ids.len(), *next),
            NodeData::String { .. } => return, // abort if this is off the edge of a string
            _ => panic!("consider_join called on non-segment node"),
        };
        let next_len = match &self.nodes[&next].data {
            NodeData::StringSegment { ids, .. } => ids.len(),
            NodeData::String { .. } => return, // abort if this is off the edge of a string
            _ => panic!("consider_join called on non-segment node"),
        };
        if segment_len >= JOIN_LEN || next_len >= JOIN_LEN || segment_len + next_len >= SPLIT_LEN {
            return;
        }
        // delete `next` and merge into this
        let deleted = self.delete_segment(next);
        let (deleted_contents, deleted_ids) = match deleted.data {
            NodeData::StringSegment { contents, ids, .. } => (contents, ids),
            _ => panic!("consider_join called on non-segment node"),
        };
        for (id, _) in &deleted_ids {
            self.id_to_node[id] = segment;
        }
        match &mut self.nodes[&segment].data {
            NodeData::StringSegment { contents, ids, .. } => {
                ids.extend(
                    deleted_ids
                        .into_iter()
                        .map(|(id, byte_opt)| (id, byte_opt.map(|n| n + contents.len()))),
                );
                contents.push_str(&deleted_contents);
            }
            _ => panic!("consider_join called on non-segment node"),
        }
    }
    /// If `segment` is greater than `SPLIT_LEN`, we'll split it into two pieces. This recurses on
    /// the children, further splitting them if they're still too long.
    // TODO this could probably be sped up to instantly segment a very long node into `n` children.
    // TODO this should call consider_join somehow so that two short things next to each other will
    // merge
    fn consider_split(&mut self, segment: NodeId) {
        let (contents, ids) = match &mut self.nodes[&segment].data {
            NodeData::StringSegment { contents, ids, .. } => (contents, ids),
            NodeData::String { .. } => return, // abort if this is off the edge of a string
            _ => panic!("consider_split called on non-segment node"),
        };
        if ids.len() <= SPLIT_LEN {
            return;
        }
        // the first index of the second segment. need to do this stuff to make sure we split
        // along a codepoint boundary
        let split_start_vec = ids.len() / 2;
        let split_start_string = ids
            .iter()
            .skip(split_start_vec)
            .find_map(|(_, byte_idx)| byte_idx.clone())
            .unwrap_or(contents.len());
        let new_string = contents.split_off(split_start_string);
        let new_ids: Vec<(Id, Option<usize>)> = ids
            .split_off(split_start_vec)
            .into_iter()
            .map(|(id, n)| (id, n.map(|n| n - split_start_string)))
            .collect();
        let new_node_id = self.insert_segment(segment);
        for (id, _) in &new_ids {
            self.id_to_node[id] = new_node_id;
        }
        match &mut self.nodes[&new_node_id].data {
            NodeData::StringSegment { contents, ids, .. } => {
                *ids = new_ids;
                *contents = new_string;
            }
            _ => panic!("insert_segment created wrong type of node"),
        }
        self.consider_split(segment);
        self.consider_split(new_node_id);
    }
    fn lookup_id_index(&self, lookup_id: &Id) -> Result<(NodeId, usize), TreeError> {
        let node_id = self
            .id_to_node
            .get(&lookup_id)
            .ok_or(TreeError::UnknownId)?;
        let node = self
            .nodes
            .get(&node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let ids = match &node.data {
            NodeData::StringSegment { ids, .. } => ids,
            _ => return Err(TreeError::UnexpectedNodeType),
        };

        for (i, (id, string_index_opt)) in ids.iter().enumerate() {
            if id == lookup_id {
                return Ok((*node_id, i));
                // don't check for string index until next iteration of loop; we want the *next*
                // char index to be the insertion point, not this one
            }
        }
        panic!("couldn't find id in list");
    }

    /// From a character id, looks up the `(containing segment id, character index, id list index)`
    /// that an appended character would need to be inserted at
    fn lookup_insertion_point(&self, lookup_id: &Id) -> Result<(NodeId, usize, usize), TreeError> {
        let node_id = self
            .id_to_node
            .get(&lookup_id)
            .ok_or(TreeError::UnknownId)?;
        let node = self
            .nodes
            .get(&node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let (ids, contents) = match &node.data {
            NodeData::StringSegment { ids, contents, .. } => (ids, contents),
            // if Id is a string, this char corresponds with the first index in the first segment
            NodeData::String { start, .. } => return Ok((*start, 0, 0)),
            _ => return Err(TreeError::UnexpectedNodeType),
        };

        let mut id_list_index_opt = None;
        for (i, (id, string_index_opt)) in ids.iter().enumerate() {
            if let Some(id_list_index) = id_list_index_opt {
                if let Some(string_index) = string_index_opt {
                    return Ok((*node_id, *string_index, id_list_index));
                }
            }
            if id == lookup_id {
                id_list_index_opt = Some(i + 1);
                // don't check for string index until next iteration of loop; we want the *next*
                // char index to be the insertion point, not this one
            }
        }
        if let Some(id_list_index) = id_list_index_opt {
            return Ok((*node_id, contents.len(), id_list_index));
        }
        panic!("id not found in segment id list");
    }

    /// Gets the type of `Id`.
    pub fn get_type(&self, id: Id) -> Result<ValueType, TreeError> {
        let node_id = self
            .id_to_node
            .get(&id)
            .ok_or(TreeError::UnknownId)?;
        let node = self
            .nodes
            .get(&node_id)
            .expect("node_id listed in id_to_node did not exist.");
        match node.data {
            NodeData::String {..} => Ok(ValueType::String),
            NodeData::StringSegment {..} => Ok(ValueType::Character),
            NodeData::Object {..} => Ok(ValueType::Object),
            NodeData::Null {..} => Ok(ValueType::Null),
            NodeData::True {..} => Ok(ValueType::True),
            NodeData::False {..} => Ok(ValueType::False),
        }
    }

    pub fn get_parent(&self, id: Id) -> Result<Option<Id>, TreeError> {
        let node_id = self
            .id_to_node
            .get(&id)
            .ok_or(TreeError::UnknownId)?;
        let node = self
            .nodes
            .get(&node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let parent_id = match node.parent {
            None => return Ok(None),
            Some(v) => v,
        };
        let parent = self
            .nodes
            .get(&parent_id)
            .expect("node_id listed in id_to_node did not exist.");
        Ok(Some(id_of_node(parent).expect("parent of node was a string segment somehow")))
    }

    fn debug_get_string(&self, id: Id) -> Result<String, TreeError> {
        let string_node_id = self
            .id_to_node
            .get(&id)
            .expect("Id passed to debug_get_string does not exist.");
        let node = self
            .nodes
            .get(&string_node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let mut next = match &node.data {
            NodeData::String { start, .. } => *start,
            _ => panic!("debug_get_string called on non-string Id"),
        };
        let mut string = String::new();
        while next != *string_node_id {
            let node = self
                .nodes
                .get(&next)
                .expect("node_id listed in segment adjacency did not exist.");
            next = match &node.data {
                NodeData::StringSegment { next, contents, .. } => {
                    string.push_str(contents);
                    *next
                }
                _ => panic!("debug_get_string called on non-string Id"),
            };
        }
        Ok(string)
    }

    /// Creates `character` in the tree with id `character_id`, and immediately inserts it after
    /// the character `append_id`. If `append_id` is the ID of a string instead of a character,
    /// `character` will be inserted at the beginning of the string. `append_id` may be a deleted
    /// character, if the tombstone is still in the tree.
    pub fn insert_character(
        &mut self,
        append_id: Id,
        character_id: Id,
        character: char,
    ) -> Result<(), TreeError> {
        if self.id_to_node.contains_key(&character_id) {
            return Err(TreeError::DuplicateId);
        }
        let (node_id, string_index, id_list_index) = self.lookup_insertion_point(&append_id)?;
        match &mut self.nodes[&node_id].data {
            NodeData::StringSegment { ids, contents, .. } => {
                contents.insert(string_index, character);
                for (_, index_opt) in ids.iter_mut().skip(id_list_index) {
                    if let Some(index) = index_opt {
                        *index += character.len_utf8();
                    }
                }
                ids.insert(id_list_index, (character_id.clone(), Some(string_index)));
                self.id_to_node.insert(character_id, node_id);
                self.consider_split(node_id);
            }
            _ => panic!("unknown object type!!"),
        }
        Ok(())
    }

    /// Deletes the character with ID `char_id`. A tombstone is left in the string, allowing future
    /// `insert_character` calls to reference this `char_id` as their `append_id`.
    pub fn delete_character(&mut self, char_id: Id) -> Result<(), TreeError> {
        let (node_id, id_list_index) = self.lookup_id_index(&char_id)?;
        match &mut self.nodes[&node_id].data {
            NodeData::StringSegment { ids, contents, .. } => {
                if let Some(old_byte_index) = ids[id_list_index].1.take() {
                    let deleted_char = contents.remove(old_byte_index);
                    for (_, byte_idx) in ids.iter_mut().skip(id_list_index) {
                        if let Some(byte_idx) = byte_idx {
                            *byte_idx -= deleted_char.len_utf8();
                        }
                    }
                }
            }
            _ => panic!("unknown object type!!"),
        }
        Ok(())
    }
}

fn id_of_node<Id: Hash + Clone + Eq + Debug>(node: &Node<Id>) -> Option<Id> {
    match &node.data {
        NodeData::String {id, ..} => Some(id.clone()),
        NodeData::Object {id, ..} => Some(id.clone()),
        NodeData::Null {id, ..} => Some(id.clone()),
        NodeData::True {id, ..} => Some(id.clone()),
        NodeData::False {id, ..} => Some(id.clone()),
        NodeData::StringSegment {..} => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct MyId(usize);

    fn num_to_char(i: usize) -> char {
        match i % 5 {
            0 => '0',
            1 => '1',
            2 => '2',
            3 => '3',
            _ => '4',
        }
    }

    #[test]
    fn object_assignment() {
        let mut tree = Tree::new_with_object_root(MyId(0));

        // {}
        // ^
        // 0
        assert_eq!(Ok(ValueType::Object), tree.get_type(MyId(0)));
        assert_eq!(Ok(None), tree.get_parent(MyId(0)));

        tree.construct_object(MyId(1)).unwrap();
        tree.object_assign(MyId(0), "my key".to_string(), Some(MyId(1)));

        tree.construct_string(MyId(2)).unwrap();
        tree.object_assign(MyId(1), "my key 2".to_string(), Some(MyId(2)));

        tree.insert_character(MyId(2), MyId(3), 'a');

        // {"my key": {"my key 2": "a"}}
        // ^          ^            ^^
        // 0          1            23
        assert_eq!(Ok(ValueType::Object), tree.get_type(MyId(0)));
        assert_eq!(Ok(ValueType::Object), tree.get_type(MyId(1)));
        assert_eq!(Ok(ValueType::String), tree.get_type(MyId(2)));
        assert_eq!(Ok(ValueType::Character), tree.get_type(MyId(3)));
        assert_eq!(Ok(None), tree.get_parent(MyId(0)));
        assert_eq!(Ok(Some(MyId(0))), tree.get_parent(MyId(1)));
        assert_eq!(Ok(Some(MyId(1))), tree.get_parent(MyId(2)));
        assert_eq!(Ok(Some(MyId(2))), tree.get_parent(MyId(3)));

        tree.construct_bool(MyId(4), true).unwrap();
        tree.object_assign(MyId(0), "my key".to_string(), Some(MyId(4)));

        // {"my key": true}
        // ^          ^
        // 0          4
        assert_eq!(Ok(ValueType::Object), tree.get_type(MyId(0)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(1)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(2)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(3)));
        assert_eq!(Ok(ValueType::True), tree.get_type(MyId(4)));
        assert_eq!(Ok(None), tree.get_parent(MyId(0)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_parent(MyId(1)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_parent(MyId(2)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_parent(MyId(3)));
        assert_eq!(Ok(Some(MyId(0))), tree.get_parent(MyId(4)));

        tree.object_assign(MyId(0), "my key".to_string(), None);

        // {}
        // ^
        // 0
        assert_eq!(Ok(ValueType::Object), tree.get_type(MyId(0)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(1)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(2)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(3)));
        assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(4)));
    }

    #[test]
    fn invalid_ids_error() {
        let mut tree = Tree::new_with_string_root(MyId(0));
        assert_eq!(tree.insert_character(MyId(0), MyId(1), 'a'), Ok(()));
        assert_eq!(
            tree.insert_character(MyId(0), MyId(1), 'a'),
            Err(TreeError::DuplicateId)
        );
        assert_eq!(
            tree.insert_character(MyId(1), MyId(0), 'a'),
            Err(TreeError::DuplicateId)
        );
        assert_eq!(
            tree.insert_character(MyId(2), MyId(5), 'a'),
            Err(TreeError::UnknownId)
        );
        assert_eq!(tree.delete_character(MyId(2)), Err(TreeError::UnknownId));
        assert_eq!(
            tree.delete_character(MyId(0)),
            Err(TreeError::UnexpectedNodeType)
        );
    }

    #[test]
    fn simple_delete() {
        let mut tree = Tree::new_with_string_root(MyId(0));
        tree.insert_character(MyId(0), MyId(1), 'a').unwrap();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok("a".to_string()));
        tree.insert_character(MyId(1), MyId(2), 'b').unwrap();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok("ab".to_string()));
        tree.delete_character(MyId(1)).unwrap();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok("b".to_string()));
        // test delete same char; should be noop
        tree.delete_character(MyId(1)).unwrap();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok("b".to_string()));
        tree.delete_character(MyId(2)).unwrap();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok("".to_string()));
    }

    #[test]
    fn insert_character() {
        let mut tree = Tree::new_with_string_root(MyId(0));
        tree.insert_character(MyId(0), MyId(1), 'a').unwrap();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok("a".to_string()));
        tree.insert_character(MyId(1), MyId(2), 'b').unwrap();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok("ab".to_string()));
        tree.insert_character(MyId(1), MyId(3), 'c').unwrap();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok("acb".to_string()));
        tree.insert_character(MyId(0), MyId(4), 'd').unwrap();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok("dacb".to_string()));
        for i in 5..10000 {
            tree.insert_character(MyId(i - 1), MyId(i), num_to_char(i))
                .unwrap();
        }

        let long_insert = (5..10000).map(|i| num_to_char(i)).collect::<String>();
        assert_eq!(tree.debug_get_string(MyId(0)), Ok(format!("d{}acb", long_insert)));
    }
}
