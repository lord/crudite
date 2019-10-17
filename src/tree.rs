//! ## Sequences
//! Sequences like arrays and strings in crudite are represented by a persistent double linked list
//! of segments. This is sorta like just the leaves of a rope connected by a doubly linked list.
//! Why not use a rope? Ropes are useful for calculating "what character is at position n" very
//! efficiently. However, it's tricky to make ropes work with random access via IDs, and there is
//! overhead for calculating the rope. We opt instead to make indexed access `O(n)` and ID-based
//! access `O(1)` by using a linked list.

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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct NodeId(usize);

/// A JSON-compatible document where each character and value in the document has a unique ID, and
/// deletions maintain tombstones for ordering future insertions.
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
    True,
    False,
    Null,
    Object {
        items: HashMap<String, NodeId>,
    },
    /// Represents a JSON string value.
    String {
        /// The first `TextSegment` in the string value. May be equal to `end` if there is only one
        /// segment.
        start: NodeId,
        /// The last `TextSegment` in the string value. May be equal to `start` if there is only
        /// one segment.
        end: NodeId,
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

    pub fn empty_string(root_id: Id) -> Self {
        let mut tree = Self::new(root_id.clone());
        tree.construct_string(root_id).unwrap();
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

    pub fn construct_bool(&mut self, id: Id, val: bool) -> Result<(), TreeError> {
        self.construct_simple(id, if val { NodeData::True } else { NodeData::False })
            .map(|_| ())
    }

    pub fn construct_null(&mut self, id: Id, val: bool) -> Result<(), TreeError> {
        self.construct_simple(id, NodeData::Null).map(|_| ())
    }

    pub fn construct_object(&mut self, id: Id, val: bool) -> Result<(), TreeError> {
        self.construct_simple(
            id,
            NodeData::Object {
                items: HashMap::new(),
            },
        )
        .map(|_| ())
    }

    pub fn construct_string(&mut self, id: Id) -> Result<(), TreeError> {
        let segment_id = self.next_id();
        let string_id = self.construct_simple(
            id,
            NodeData::String {
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
            NodeData::True
            | NodeData::False
            | NodeData::Null
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
                NodeData::True | NodeData::False | NodeData::Null => {
                    // do nothing
                }
                NodeData::Object { items } => {
                    for (_, id) in items {
                        queue.push(id);
                    }
                }
                NodeData::String { start, .. } => {
                    queue.push(start);
                }
                NodeData::StringSegment { next, .. } => {
                    queue.push(next);
                }
            }
        }
    }

    // fn object_assign(&mut self, object: Id, key: String, value: Id) {
    //     let node_id = self
    //         .id_to_node
    //         .get(&lookup_id)
    //         .ok_or(TreeError::UnknownId)?;
    //     match self.nodes[node_id].data {
    //         NodeData::Object {items} => {
    //         }
    //         _ => return Err(TreeError::UnexpectedNodeType),
    //     }
    // }

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

    pub fn get_string(&self, id: Id) -> Result<String, TreeError> {
        let string_node_id = self
            .id_to_node
            .get(&id)
            .expect("Id passed to get_string does not exist.");
        let node = self
            .nodes
            .get(&string_node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let mut next = match &node.data {
            NodeData::String { start, .. } => *start,
            _ => panic!("get_string called on non-string Id"),
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
                _ => panic!("get_string called on non-string Id"),
            };
        }
        Ok(string)
    }

    pub fn insert_character(
        &mut self,
        append_id: Id,
        this_id: Id,
        character: char,
    ) -> Result<(), TreeError> {
        if self.id_to_node.contains_key(&this_id) {
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
                ids.insert(id_list_index, (this_id.clone(), Some(string_index)));
                self.id_to_node.insert(this_id, node_id);
                self.consider_split(node_id);
            }
            _ => panic!("unknown object type!!"),
        }
        Ok(())
    }

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
    fn invalid_ids_error() {
        let mut tree = Tree::empty_string(MyId(0));
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
        let mut tree = Tree::empty_string(MyId(0));
        tree.insert_character(MyId(0), MyId(1), 'a').unwrap();
        assert_eq!(tree.get_string(MyId(0)), Ok("a".to_string()));
        tree.insert_character(MyId(1), MyId(2), 'b').unwrap();
        assert_eq!(tree.get_string(MyId(0)), Ok("ab".to_string()));
        tree.delete_character(MyId(1)).unwrap();
        assert_eq!(tree.get_string(MyId(0)), Ok("b".to_string()));
        // test delete same char; should be noop
        tree.delete_character(MyId(1)).unwrap();
        assert_eq!(tree.get_string(MyId(0)), Ok("b".to_string()));
        tree.delete_character(MyId(2)).unwrap();
        assert_eq!(tree.get_string(MyId(0)), Ok("".to_string()));
    }

    #[test]
    fn insert_character() {
        let mut tree = Tree::empty_string(MyId(0));
        tree.insert_character(MyId(0), MyId(1), 'a').unwrap();
        assert_eq!(tree.get_string(MyId(0)), Ok("a".to_string()));
        tree.insert_character(MyId(1), MyId(2), 'b').unwrap();
        assert_eq!(tree.get_string(MyId(0)), Ok("ab".to_string()));
        tree.insert_character(MyId(1), MyId(3), 'c').unwrap();
        assert_eq!(tree.get_string(MyId(0)), Ok("acb".to_string()));
        tree.insert_character(MyId(0), MyId(4), 'd').unwrap();
        assert_eq!(tree.get_string(MyId(0)), Ok("dacb".to_string()));
        for i in 5..10000 {
            tree.insert_character(MyId(i - 1), MyId(i), num_to_char(i))
                .unwrap();
        }

        let long_insert = (5..10000).map(|i| num_to_char(i)).collect::<String>();
        assert_eq!(tree.get_string(MyId(0)), Ok(format!("d{}acb", long_insert)));
    }
}
