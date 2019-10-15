//! ## Sequences
//! Sequences like arrays and strings in crudite are represented by a persistent double linked list
//! of segments. This is sorta like just the leaves of a rope connected by a doubly linked list.
//! Why not use a rope? Ropes are useful for calculating "what character is at position n" very
//! efficiently. However, it's tricky to make ropes work with random access via IDs, and there is
//! overhead for calculating the rope. We opt instead to make indexed access `O(n)` and ID-based
//! access `O(1)` by using a linked list.

use std::hash::Hash;
use im::{Vector, HashMap};

const JOIN_LEN: usize = 511;
const SPLIT_LEN: usize = 1024;

/// Tree represents a JSON-compatible document.
type NodeId = usize;
#[derive(Clone)]
pub struct Tree<Id: Hash + Clone + Eq> {
    /// Number to use for the next node that is created.
    next_node: NodeId,

    /// Maps external IDs to their position in the tree. In the case of Segments of a sequence,
    /// futher disambiguation may be necessary to find the exact character this represents within
    /// the string.
    id_to_node: HashMap<Id, NodeId>,

    /// Maps node ids to node data.
    nodes: HashMap<NodeId, Node<Id>>,
}

#[derive(Clone)]
struct Node<Id: Hash + Clone + Eq> {
    data: NodeData<Id>,
    parent: Option<NodeId>,
}

#[derive(Clone)]
enum NodeData<Id: Hash + Clone + Eq> {
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

impl <Id: Hash + Clone + Eq> Tree<Id> {
    pub fn new() -> Self {
        Tree {
            next_node: 0,
            id_to_node: HashMap::new(),
            nodes: HashMap::new(),
        }
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
            NodeData::StringSegment{prev, next, ..} => (*prev, *next),
            _ => panic!("delete_segment called on non-segment node"),
        };
        if old_prev == old_next {
            // TODO should this actually panic?
            panic!("attempted to delete only segment in list");
        }
        match &mut self.nodes[&old_prev].data {
            NodeData::StringSegment{next, ..} => *next = old_next,
            NodeData::String{start, ..} => *start = old_next,
            _ => panic!("delete_segment called on non-segment node"),
        }
        match &mut self.nodes[&old_next].data {
            NodeData::StringSegment{prev, ..} => *prev = old_prev,
            NodeData::String{end, ..} => *end = old_prev,
            _ => panic!("delete_segment called on non-segment node"),
        }
        segment_data
    }

    // Inserts a new, empty segment after `append_to`, and returns the usize of the new node.
    fn insert_segment(&mut self, append_to: NodeId) -> usize {
        let new_id = self.next_id();
        let (parent, prev, next) = match &mut self.nodes[&append_to] {
            Node{parent, data: NodeData::StringSegment{prev, next, ..}} => {
                let old_next = *next;
                *next = new_id;
                (*parent, append_to, old_next)
            },
            Node{parent: _, data: NodeData::String{start, ..}} => {
                let old_start = *start;
                *start = new_id;
                (Some(append_to), append_to, old_start)
            },
            _ => panic!("insert_segment called on non-segment node"),
        };
        let node = Node {
            parent,
            data: NodeData::StringSegment {
                prev,
                next,
                contents: String::new(),
                ids: Vec::new(),
            }
        };
        self.nodes[&new_id] = node;
        match &mut self.nodes[&next].data {
            NodeData::StringSegment{prev, ..} => {
                *prev = new_id;
            },
            NodeData::String{end, ..} => {
                *end = new_id;
            },
            _ => panic!("insert_segment called on non-segment node"),
        }
        new_id
    }

    /// If either `segment` or the next node are less than `JOIN_LEN`, and together they are
    /// less than `SPLIT_LEN`, then this function joins them together. In all other cases, it is a
    /// no-op.
    fn consider_join(&mut self, segment: usize) {
        let (segment_len, next) = match &self.nodes[&segment].data {
            NodeData::StringSegment{contents, next, ..} => (contents.len(), *next),
            NodeData::String{..} => return, // abort if this is off the edge of a string
            _ => panic!("consider_join called on non-segment node"),
        };
        let next_len = match &self.nodes[&next].data {
            NodeData::StringSegment{contents, ..} => contents.len(),
            NodeData::String{..} => return, // abort if this is off the edge of a string
            _ => panic!("consider_join called on non-segment node"),
        };
        if segment_len >= JOIN_LEN || next_len >= JOIN_LEN || segment_len+next_len >= SPLIT_LEN {
            return
        }
        // delete `next` and merge into this
        let deleted = self.delete_segment(next);
        let (deleted_contents, deleted_ids) = match deleted.data {
            NodeData::StringSegment{contents, ids, ..} => (contents, ids),
            _ => panic!("consider_join called on non-segment node"),
        };
        for (id, _) in &deleted_ids {
            self.id_to_node[id] = segment;
        }
        match &mut self.nodes[&segment].data {
            NodeData::StringSegment{contents, ids, ..} => {
                contents.push_str(&deleted_contents);
                let segment_id_count = ids.len();
                ids.extend(deleted_ids.into_iter().map(|(id, byte_opt)| (id, byte_opt.map(|n| n + segment_id_count))));
            },
            _ => panic!("consider_join called on non-segment node"),
        }
    }
    /// If `segment` is greater than `SPLIT_LEN`, we'll split it into two pieces. We'll also then
    /// check if either of the resulting segments is short enough to merge with the next segment
    /// over. If the resulting split strings are too long, `consider_split` will recurse on the new
    /// children.
    // TODO this could probably be sped up to instantly segment a very long node into `n` children.
    fn consider_split(&mut self, segment: usize) {
        let contents = match &self.nodes[&segment].data {
            NodeData::StringSegment{contents, ..} => contents,
            NodeData::String{..} => return, // abort if this is off the edge of a string
            _ => panic!("consider_split called on non-segment node"),
        };
        let len = contents.len();
        if len <= SPLIT_LEN {
            return
        }
        // the first index of the second segment. need to do this stuff to make sure we split
        // along a codepoint boundary
        let split_start = contents.char_indices().find(|(i, _)| *i >= len/2).expect("somehow we failed to find a split point for the string. maybe SPLIT_LEN is really small?");
    }

    // pub fn insert_character(&mut self, id: Id, character: char) -> Self {
    // }
}


#[cfg(test)]
mod test {
    #[test]
    fn test_merge_leaves() {
    }
}
