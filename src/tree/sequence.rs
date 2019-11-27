use std::hash::Hash;
use std::fmt::Debug;
use super::{Tree, Child, NodeData, NodeId, TreeError, Node};

const JOIN_LEN: usize = 511;
const SPLIT_LEN: usize = 1024;

/// Creates `character` in the tree with id `character_id`, and immediately inserts it after
/// the character `append_id`. If `append_id` is the ID of a string instead of a character,
/// `character` will be inserted at the beginning of the string. `append_id` may be a deleted
/// character, if the tombstone is still in the tree.
pub(super) fn insert_character<Id: Hash + Clone + Eq + Debug>(
    tree: &mut Tree<Id>,
    append_id: Id,
    character_id: Id,
    character: char,
) -> Result<(), TreeError> {
    if tree.id_to_node.contains_key(&character_id) {
        return Err(TreeError::DuplicateId);
    }
    let (node_id, string_index, id_list_index) = lookup_insertion_point(tree, &append_id)?;
    match &mut tree.nodes[&node_id].data {
        NodeData::StringSegment { ids, contents, .. } => {
            contents.insert(string_index, character);
            for (_, index_opt) in ids.iter_mut().skip(id_list_index) {
                if let Some(index) = index_opt {
                    *index += character.len_utf8();
                }
            }
            ids.insert(id_list_index, (character_id.clone(), Some(string_index)));
            tree.id_to_node.insert(character_id, node_id);
            let (left, right) = consider_split(tree, node_id);
            consider_join(tree, left, false);
            consider_join(tree, right, false);
        }
        _ => panic!("unknown object type!!"),
    }
    Ok(())
}

/// Deletes the character with ID `char_id`. A tombstone is left in the string, allowing future
/// `insert_character` calls to reference this `char_id` as their `append_id`.
pub(super) fn delete_character<Id: Hash + Clone + Eq + Debug>(tree: &mut Tree<Id>, char_id: Id) -> Result<(), TreeError> {
    let (node_id, id_list_index) = lookup_id_index(tree, &char_id)?;
    match &mut tree.nodes[&node_id].data {
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

// Inserts a new, empty segment after `append_to`, and returns the usize of the new node.
fn insert_segment<Id: Hash + Clone + Eq + Debug>(tree: &mut Tree<Id>, append_to: NodeId) -> NodeId {
    let new_id = tree.next_id();
    let (parent, prev, next) = match &mut tree.nodes[&append_to] {
        Node {
            parent,
            data: NodeData::StringSegment { next, .. },
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
    tree.nodes.insert(new_id, node);
    match &mut tree.nodes[&next].data {
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

/// Deletes a segment with node id `usize`, returns deleted NodeData and new Tree. Caller is
/// responsible for updating `id_to_node`, but this takes care of updating `next`, `prev`, or
/// if necessary, `start` and `end`. If this is the only segment in the list, it will panic.
fn delete_segment<Id: Hash + Clone + Eq + Debug>(tree: &mut Tree<Id>, segment: NodeId) -> Node<Id> {
    let segment_data = tree.nodes.remove(&segment).expect("segment did not exist");
    let (old_prev, old_next) = match &segment_data.data {
        NodeData::StringSegment { prev, next, .. } => (*prev, *next),
        _ => panic!("delete_segment called on non-segment node"),
    };
    if old_prev == old_next {
        // TODO should this actually panic?
        panic!("attempted to delete only segment in list");
    }
    match &mut tree.nodes[&old_prev].data {
        NodeData::StringSegment { next, .. } => *next = old_next,
        NodeData::String { start, .. } => *start = old_next,
        _ => panic!("delete_segment called on non-segment node"),
    }
    match &mut tree.nodes[&old_next].data {
        NodeData::StringSegment { prev, .. } => *prev = old_prev,
        NodeData::String { end, .. } => *end = old_prev,
        _ => panic!("delete_segment called on non-segment node"),
    }
    segment_data
}

fn lookup_id_index<Id: Hash + Clone + Eq + Debug>(tree: &Tree<Id>, lookup_id: &Id) -> Result<(NodeId, usize), TreeError> {
    let node_id = tree
        .id_to_node
        .get(&lookup_id)
        .ok_or(TreeError::UnknownId)?;
    let node = tree
        .nodes
        .get(&node_id)
        .expect("node_id listed in id_to_node did not exist.");
    let ids = match &node.data {
        NodeData::StringSegment { ids, .. } => ids,
        _ => return Err(TreeError::UnexpectedNodeType),
    };

    for (i, (id, _)) in ids.iter().enumerate() {
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
fn lookup_insertion_point<Id: Hash + Clone + Eq + Debug>(tree: &Tree<Id>, lookup_id: &Id) -> Result<(NodeId, usize, usize), TreeError> {
    let node_id = tree
        .id_to_node
        .get(&lookup_id)
        .ok_or(TreeError::UnknownId)?;
    let node = tree
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

/// If either `segment` or the next node are less than `JOIN_LEN`, and together they are
/// less than `SPLIT_LEN`, then this function joins them together. In all other cases, it is a
/// no-op.
fn consider_join<Id: Hash + Clone + Eq + Debug>(tree: &mut Tree<Id>, segment: NodeId, rightward: bool) {
    let (left, right) = match (&tree.nodes[&segment].data, rightward) {
        (NodeData::String { .. }, _) => return, // abort if this is off the edge of a string
        (NodeData::StringSegment { next, .. }, true) => (segment, *next),
        (NodeData::StringSegment { prev, .. }, false) => (*prev, segment),
        _ => panic!("consider_join called on non-segment node"),
    };
    let left_len = match &tree.nodes[&left].data {
        NodeData::StringSegment { ids, .. } => ids.len(),
        NodeData::String { .. } => return, // abort if this is off the edge of a string
        _ => panic!("consider_join called on non-segment node"),
    };
    let right_len = match &tree.nodes[&right].data {
        NodeData::StringSegment { ids, .. } => ids.len(),
        NodeData::String { .. } => return,
        _ => panic!("consider_join called on non-segment node"),
    };
    if left_len >= JOIN_LEN || right_len >= JOIN_LEN || left_len + right_len >= SPLIT_LEN {
        return;
    }
    // delete `right` and merge into this
    let deleted = delete_segment(tree, right);
    let (deleted_contents, deleted_ids) = match deleted.data {
        NodeData::StringSegment { contents, ids, .. } => (contents, ids),
        _ => panic!("consider_join called on non-segment node"),
    };
    for (id, _) in &deleted_ids {
        tree.id_to_node[id] = left;
    }
    match &mut tree.nodes[&left].data {
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
/// the children, further splitting them if they're still too long. Returns the leftmost and
/// rightmost of the new segments; if no split occured, these will both still be `segment`.
// TODO this could probably be sped up to instantly segment a very long node into `n` children.
fn consider_split<Id: Hash + Clone + Eq + Debug>(tree: &mut Tree<Id>, segment: NodeId) -> (NodeId, NodeId) {
    let (contents, ids) = match &mut tree.nodes[&segment].data {
        NodeData::StringSegment { contents, ids, .. } => (contents, ids),
        NodeData::String { .. } => return (segment, segment), // abort if this is off the edge of a string
        _ => panic!("consider_split called on non-segment node"),
    };
    if ids.len() <= SPLIT_LEN {
        return (segment, segment);
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
    let new_node_id = insert_segment(tree, segment);
    for (id, _) in &new_ids {
        tree.id_to_node[id] = new_node_id;
    }
    match &mut tree.nodes[&new_node_id].data {
        NodeData::StringSegment { contents, ids, .. } => {
            *ids = new_ids;
            *contents = new_string;
        }
        _ => panic!("insert_segment created wrong type of node"),
    }
    let (left, _) = consider_split(tree, segment);
    let (_, right) = consider_split(tree, new_node_id);
    (left, right)
}
