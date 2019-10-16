//! ## Sequences
//! Sequences like arrays and strings in crudite are represented by a persistent double linked list
//! of segments. This is sorta like just the leaves of a rope connected by a doubly linked list.
//! Why not use a rope? Ropes are useful for calculating "what character is at position n" very
//! efficiently. However, it's tricky to make ropes work with random access via IDs, and there is
//! overhead for calculating the rope. We opt instead to make indexed access `O(n)` and ID-based
//! access `O(1)` by using a linked list.

use im::{HashMap, Vector};
use std::hash::Hash;
use std::fmt::Debug;

const JOIN_LEN: usize = 511;
const SPLIT_LEN: usize = 1024;

/// Tree represents a JSON-compatible document.
type NodeId = usize;
#[derive(Clone, Debug)]
pub struct Tree<Id: Hash + Clone + Eq + Debug> {
    /// Number to use for the next node that is created.
    next_node: NodeId,

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
        items: HashMap<String, usize>,
    },
    /// Represents a JSON string value.
    String {
        /// The first `TextSegment` in the string value. May be equal to `end` if there is only one
        /// segment.
        start: usize,
        /// The last `TextSegment` in the string value. May be equal to `start` if there is only
        /// one segment.
        end: usize,
    },
    /// Represents a range of a JSON string value.
    StringSegment {
        /// Node index of the previous `TextSegment` in this string. If this is the first segment
        /// in the string, refers to the `Text` parent.
        prev: usize,
        /// Node index of the next `TextSegment` in this string. If this is the last segment
        /// in the string, refers to the `Text` parent.
        next: usize,
        /// String contents of this segment.
        contents: String,
        /// List of ids. If they are a tombstone, the the Option will be None, if they represent a
        /// live character, the Option will show the index of the character.
        ids: Vec<(Id, Option<usize>)>,
    },
}

impl<Id: Hash + Clone + Eq + Debug> Tree<Id> {
    fn new(root_id: Id, root_node: NodeData<Id>) -> Self {
        let mut tree = Tree {
            next_node: 1,
            id_to_node: HashMap::new(),
            nodes: HashMap::new(),
        };
        tree.id_to_node.insert(root_id, 0);
        tree.nodes.insert(0, Node {
            parent: None,
            data: root_node,
        });
        tree
    }

    pub fn empty_string(root_id: Id) -> Self {
        let mut tree = Self::new(root_id, NodeData::String {
            start: 1,
            end: 1,
        });
        tree.nodes.insert(1, Node {
            parent: Some(0),
            data: NodeData::StringSegment {
                next: 0,
                prev: 0,
                ids: vec![],
                contents: String::new(),
            }
        });
        tree.next_node += 1;
        tree
    }

    fn next_id(&mut self) -> usize {
        let res = self.next_node;
        self.next_node += 1;
        res
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
    fn insert_segment(&mut self, append_to: NodeId) -> usize {
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
    fn consider_join(&mut self, segment: usize) {
        let (segment_len, next) = match &self.nodes[&segment].data {
            NodeData::StringSegment { contents, next, .. } => (contents.len(), *next),
            NodeData::String { .. } => return, // abort if this is off the edge of a string
            _ => panic!("consider_join called on non-segment node"),
        };
        let next_len = match &self.nodes[&next].data {
            NodeData::StringSegment { contents, .. } => contents.len(),
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
    fn consider_split(&mut self, segment: usize) {
        let (contents, ids) = match &mut self.nodes[&segment].data {
            NodeData::StringSegment { contents, ids, .. } => (contents, ids),
            NodeData::String { .. } => return, // abort if this is off the edge of a string
            _ => panic!("consider_split called on non-segment node"),
        };
        let len = contents.len();
        if len <= SPLIT_LEN {
            return;
        }
        // the first index of the second segment. need to do this stuff to make sure we split
        // along a codepoint boundary
        let (split_start_string, _) = contents
            .char_indices()
            .find(|(i, _)| *i >= len / 2)
            .expect("somehow we failed to find a split point for the string.");
        let (split_start_vec, _) = ids
            .iter()
            .enumerate()
            .find(|(_, (_, byte_i))| *byte_i == Some(split_start_string))
            .expect("somehow failed to find a split point for ids");
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
    // TODO i think we should have the nodes be based on length of lookup insertion point instead
    // of length of string to avoid bad cases where there are massive clones?? maybe

    /// From a character id, looks up the `(containing segment id, character index, id list index)`
    /// that an appended character would need to be inserted at
    fn lookup_insertion_point(&self, lookup_id: &Id) -> (NodeId, usize, usize) {
        let node_id = self
            .id_to_node
            .get(&lookup_id)
            .expect("Id passed to lookup_character does not exist.");
        let node = self
            .nodes
            .get(&node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let (ids, contents) = match &node.data {
            NodeData::StringSegment { ids, contents, .. } => (ids, contents),
            // if Id is a string, this char corresponds with the first index in the first segment
            NodeData::String { start, .. } => return (*start, 0, 0),
            _ => panic!("lookup_character called on non-character Id"),
        };

        let mut id_list_index_opt = None;
        for (i, (id, string_index_opt)) in ids.iter().enumerate() {
            if let Some(id_list_index) = id_list_index_opt {
                if let Some(string_index) = string_index_opt {
                    return (*node_id, *string_index, id_list_index);
                }
            }
            if id == lookup_id {
                id_list_index_opt = Some(i+1);
                // don't check for string index until next iteration of loop; we want the *next*
                // char index to be the insertion point, not this one
            }
        }
        if let Some(id_list_index) = id_list_index_opt {
            return (*node_id, contents.len(), id_list_index)
        }
        panic!("id not found in segment id list");
    }

    pub fn get_string(&self, id: Id) -> String {
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
                },
                _ => panic!("get_string called on non-string Id"),
            };
        }
        string
    }

    // TODO since untrusted code is going in here, should make invalid Ids return an error instead
    pub fn insert_character(&mut self, append_id: Id, this_id: Id, character: char) {
        let (node_id, string_index, id_list_index) = self.lookup_insertion_point(&append_id);
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
            },
            _ => panic!("unknown object type!!"),
        }
    }
}

// TODO should double check ids were not already taken?

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
    fn blah() {
        let mut tree = Tree::empty_string(MyId(0));
        tree.insert_character(MyId(0), MyId(1), 'a');
        assert_eq!(tree.get_string(MyId(0)), "a");
        tree.insert_character(MyId(1), MyId(2), 'b');
        assert_eq!(tree.get_string(MyId(0)), "ab");
        tree.insert_character(MyId(1), MyId(3), 'c');
        assert_eq!(tree.get_string(MyId(0)), "acb");
        tree.insert_character(MyId(0), MyId(4), 'd');
        assert_eq!(tree.get_string(MyId(0)), "dacb");
        for i in 5..10000 {
            tree.insert_character(MyId(i-1), MyId(i), num_to_char(i));
        }

        let long_insert = (5..10000).map(|i| num_to_char(i)).collect::<String>();
        assert_eq!(tree.get_string(MyId(0)), format!("d{}acb", long_insert));
    }
}
