mod sequence;
#[cfg(test)]
mod test;

use im::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value<Id> {
    True,
    False,
    Int(i64),
    Null,
    Collection(Id),
    Unset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Child {
    True,
    False,
    Int(i64),
    Null,
    Collection(NodeId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit<Id> {
    ListCreate {
        /// id of new list
        id: Id,
    },
    ListInsert {
        /// If new item is at start of list, `prev` is the parent list object. otherwise, it's a
        /// id specified in a previous `ListInsert` operation.
        prev: Id,
        /// Insertion id. This is used for deleting list items, and in other `ListInsert`'s `prev`.
        id: Id,
        /// Item to be inserted. If this item had a prevous parent, it is removed from that parent.
        item: Value<Id>,
    },
    ListDelete {
        /// Id of character to delete
        id: Id,
    },
    MapCreate {
        /// id of new map
        id: Id,
    },
    MapInsert {
        /// Id of parent map
        parent: Id,
        /// Key of item in hashmap
        key: String,
        /// Item to be set. If this item had a prevous parent, it is removed from that parent.
        item: Value<Id>,
    },
    TextCreate {
        /// id of new text
        id: Id,
    },
    TextInsert {
        /// If new item is at start of text, `prev` is the parent text object. otherwise, it's a
        /// id specified in a previous `TextInsert` operation.
        prev: Id,
        /// Id of newly created character
        id: Id,
        /// Actual new character value
        character: char,
    },
    TextDelete {
        /// Id of character to delete
        id: Id,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeType {
    String,
    Character,
    Object,
    List,
    ListEntry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeError {
    UnknownId,
    UnexpectedNodeType,
    DuplicateId,
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
    Object {
        items: HashMap<String, Child>,
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

impl<Id: Hash + Clone + Eq + Debug> Node<Id> {
    fn id(&self) -> Option<Id> {
        match &self.data {
            NodeData::String { id, .. } => Some(id.clone()),
            NodeData::Object { id, .. } => Some(id.clone()),
            NodeData::StringSegment { .. } => None,
        }
    }

    /// Creates a new, empty NodeData for a segment with the same kind. `prev` and `next` are
    /// expected to be overwritten by the calling function.
    fn segment_create(&self) -> NodeData<Id> {
        match &self.data {
            NodeData::StringSegment { prev, next, .. } => NodeData::StringSegment {
                    prev: *prev,
                    next: *next,
                    contents: String::new(),
                    ids: Vec::new(),
            },
            NodeData::String { end, start, .. } => NodeData::StringSegment {
                    prev: *end,
                    next: *start,
                    contents: String::new(),
                    ids: Vec::new(),
            },
            _ => panic!("segment_create called on non-sequence node"),
        }
    }

    /// Returns (prev, next) for segments, and (end, start) for sequence containers
    fn segment_adjacencies(&mut self) -> (&mut NodeId, &mut NodeId) {
        match &mut self.data {
            NodeData::String { end, start, .. } => (end, start),
            NodeData::StringSegment { prev, next, .. } => (prev, next),
            _ => panic!("segment_adjacencies called on non-sequence typed node"),
        }
    }

    fn segment_ids(&self) -> Result<&Vec<(Id, Option<usize>)>, TreeError> {
        match &self.data {
            NodeData::StringSegment { ids, .. } => Ok(ids),
            _ => Err(TreeError::UnexpectedNodeType),
        }
    }

    fn segment_ids_mut(&mut self) -> Result<&mut Vec<(Id, Option<usize>)>, TreeError> {
        match &mut self.data {
            NodeData::StringSegment { ids, .. } => Ok(ids),
            _ => Err(TreeError::UnexpectedNodeType),
        }
    }
}

impl<Id: Hash + Clone + Eq + Debug> Tree<Id> {
    /// This is private since it constructs a tree with no root value; use one of the public
    /// constructors to create the `Tree` instead.
    pub fn new(root_id: Id) -> Self {
        Tree {
            next_node: NodeId(0),
            id_to_node: HashMap::new(),
            nodes: HashMap::new(),
            root: root_id,
        }
    }

    pub fn update(&mut self, edit: &Edit<Id>) -> Result<(), TreeError> {
        match edit {
            Edit::ListCreate { id: _ } => unimplemented!(),
            Edit::ListInsert {
                prev: _,
                id: _,
                item: _,
            } => unimplemented!(),
            Edit::ListDelete { id: _ } => unimplemented!(),
            Edit::MapCreate { id } => self.construct_object(id.clone()),
            Edit::MapInsert { parent, key, item } => {
                self.object_assign(parent.clone(), key.clone(), item.clone())
            }
            Edit::TextCreate { id } => self.construct_string(id.clone()),
            Edit::TextInsert {
                prev,
                id,
                character,
            } => self.insert_character(prev.clone(), id.clone(), *character),
            Edit::TextDelete { id } => self.delete_character(id.clone()),
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
            NodeData::Object { .. } | NodeData::String { .. } => { /* do nothing */ }
            _ => panic!("attempted to delete invalid type"),
        }
        let mut queue = vec![item];
        while let Some(item) = queue.pop() {
            let node = match self.nodes.remove(&item) {
                Some(v) => v,
                None => continue,
            };
            match node.data {
                NodeData::Object { id, items } => {
                    for (_, val) in items {
                        match val {
                            Child::Collection(id) => {
                                queue.push(id);
                            }
                            // do nothing for other values; don't have any subchildren to delete
                            Child::True | Child::False | Child::Null | Child::Int(_) => {}
                        }
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
    /// Moves `value` to `object[key]`. If `value` is `None`, the key is deleted. The previous
    /// value must be deleted, which takes `O(log n)` for each item in the subtree of the previous
    /// value.
    pub fn object_assign(
        &mut self,
        object: Id,
        key: String,
        value: Value<Id>,
    ) -> Result<(), TreeError> {
        let object_node_id = *self.id_to_node.get(&object).ok_or(TreeError::UnknownId)?;
        let child_opt = match value {
            Value::Collection(id) => {
                let node_id = *self.id_to_node.get(&id).ok_or(TreeError::UnknownId)?;
                Some(Child::Collection(node_id))
            }
            Value::True => Some(Child::True),
            Value::False => Some(Child::False),
            Value::Null => Some(Child::Null),
            Value::Int(i) => Some(Child::Int(i)),
            Value::Unset => None,
        };
        if let Some(Child::Collection(child)) = &child_opt {
            self.nodes[&child].parent = Some(object_node_id);
        }
        match &mut self.nodes[&object_node_id].data {
            NodeData::Object { items, id: _ } => {
                let old = if let Some(child) = child_opt {
                    items.insert(key, child)
                } else {
                    items.remove(&key)
                };
                if let Some(Child::Collection(old_id)) = old {
                    self.delete(old_id);
                }
            }
            _ => return Err(TreeError::UnexpectedNodeType),
        }
        Ok(())
    }

    pub fn object_get(&self, object: Id, key: &str) -> Result<Value<Id>, TreeError> {
        let object_node_id = *self.id_to_node.get(&object).ok_or(TreeError::UnknownId)?;
        let child = match &self.nodes[&object_node_id].data {
            NodeData::Object { items, id: _ } => {
                if let Some(child) = items.get(key) {
                    child
                } else {
                    return Ok(Value::Unset);
                }
            }
            _ => return Err(TreeError::UnexpectedNodeType),
        };
        let val = match child {
            Child::True => Value::True,
            Child::False => Value::False,
            Child::Null => Value::Null,
            Child::Int(i) => Value::Int(*i),
            Child::Collection(node_id) => {
                let id = self.nodes[&node_id]
                    .id()
                    .expect("segment was somehow child of object?");
                Value::Collection(id)
            }
        };
        Ok(val)
    }

    /// Gets the type of `Id`.
    pub fn get_type(&self, id: Id) -> Result<NodeType, TreeError> {
        let node_id = self.id_to_node.get(&id).ok_or(TreeError::UnknownId)?;
        let node = self
            .nodes
            .get(&node_id)
            .expect("node_id listed in id_to_node did not exist.");
        match node.data {
            NodeData::String { .. } => Ok(NodeType::String),
            NodeData::StringSegment { .. } => Ok(NodeType::Character),
            NodeData::Object { .. } => Ok(NodeType::Object),
        }
    }

    pub fn get_parent(&self, id: Id) -> Result<Option<Id>, TreeError> {
        let node_id = self.id_to_node.get(&id).ok_or(TreeError::UnknownId)?;
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
        Ok(Some(
            parent
                .id()
                .expect("parent of node was a string segment somehow"),
        ))
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
        sequence::insert_character(self, append_id, character_id, character)
    }

    /// Deletes the character with ID `char_id`. A tombstone is left in the string, allowing future
    /// `insert_character` calls to reference this `char_id` as their `append_id`.
    pub fn delete_character(&mut self, char_id: Id) -> Result<(), TreeError> {
        sequence::delete_character(self, char_id)
    }
}
