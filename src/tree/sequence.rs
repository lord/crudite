use std::hash::Hash;
use std::fmt::Debug;
use super::{Tree, Child, NodeData, NodeId};

const JOIN_LEN: usize = 511;
const SPLIT_LEN: usize = 1024;

/// If either `segment` or the next node are less than `JOIN_LEN`, and together they are
/// less than `SPLIT_LEN`, then this function joins them together. In all other cases, it is a
/// no-op.
pub(super) fn consider_join<Id: Hash + Clone + Eq + Debug>(tree: &mut Tree<Id>, segment: NodeId, rightward: bool) {
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
    let deleted = tree.delete_segment(right);
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
pub(super) fn consider_split<Id: Hash + Clone + Eq + Debug>(tree: &mut Tree<Id>, segment: NodeId) -> (NodeId, NodeId) {
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
    let new_node_id = tree.insert_segment(segment);
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
