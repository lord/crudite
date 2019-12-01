use super::{Node, NodeId, Tree, TreeError};
use std::fmt::Debug;
use std::hash::Hash;

const SPLIT_LEN: usize = 1024;

/// `insert_fn(index to insert in contents at, node to insert into) -> length of inserted item`
pub(super) fn insert<Id: Hash + Clone + Eq + Debug, F: FnOnce(usize, &mut Node<Id>) -> usize>(
    tree: &mut Tree<Id>,
    append_id: Id,
    character_id: Id,
    insert_fn: F,
) -> Result<(), TreeError> {
    if tree.id_to_node.contains_key(&character_id) {
        return Err(TreeError::DuplicateId);
    }
    let (node_id, string_index, id_list_index) = lookup_insertion_point(tree, &append_id)?;
    let insert_len = insert_fn(string_index, &mut tree.nodes[&node_id]);
    let ids = tree.nodes[&node_id].segment_ids_mut()?;
    // contents.insert(string_index, character);
    for (_, index_opt) in ids.iter_mut().skip(id_list_index) {
        if let Some(index) = index_opt {
            *index += insert_len;
        }
    }
    tree.id_to_node.insert(character_id.clone(), node_id);
    ids.insert(id_list_index, (character_id, Some(string_index)));
    consider_split(tree, node_id);
    Ok(())
}

pub(super) fn delete<Id: Hash + Clone + Eq + Debug, F: FnOnce(usize, &mut Node<Id>) -> usize>(
    tree: &mut Tree<Id>,
    char_id: Id,
    delete_fn: F,
) -> Result<(), TreeError> {
    let (node_id, id_list_index) = lookup_id_index(tree, &char_id)?;
    if let Some(old_byte_index) = tree.nodes[&node_id].segment_ids_mut()?[id_list_index]
        .1
        .take()
    {
        let delete_len = delete_fn(old_byte_index, &mut tree.nodes[&node_id]);
        for (_, byte_idx) in tree.nodes[&node_id]
            .segment_ids_mut()?
            .iter_mut()
            .skip(id_list_index)
        {
            if let Some(byte_idx) = byte_idx {
                *byte_idx -= delete_len;
            }
        }
    }
    Ok(())
}

// Inserts a new, empty segment after `to_split`, and returns the usize of the new node.
fn insert_segment<Id: Hash + Clone + Eq + Debug>(
    tree: &mut Tree<Id>,
    to_split: NodeId,
    id_split_index: usize,
) -> NodeId {
    let new_id = tree.next_id();
    // split old node; insert into tree
    {
        let parent = tree.nodes[&to_split].parent;
        let mut node = Node {
            parent: parent,
            data: tree.nodes[&to_split].segment_create(),
        };
        let contents_len = tree.nodes[&to_split].segment_contents_len().unwrap();
        let split_start_string = tree.nodes[&to_split]
            .segment_ids_mut()
            .unwrap()
            .iter()
            .skip(id_split_index)
            .find_map(|(_, byte_idx)| byte_idx.clone())
            .unwrap_or(contents_len);
        let new_ids: Vec<(Id, Option<usize>)> = tree.nodes[&to_split]
            .segment_ids_mut()
            .unwrap()
            .split_off(id_split_index)
            .into_iter()
            .map(|(id, n)| (id, n.map(|n| n - split_start_string)))
            .collect();
        tree.nodes[&to_split].segment_split_contents_into(&mut node, split_start_string);
        for (id, _) in &new_ids {
            tree.id_to_node[id] = new_id;
        }
        *node.segment_ids_mut().unwrap() = new_ids;
        tree.nodes.insert(new_id, node);
    }

    // adjust to_split, which is the segment before new_id
    let old_to_split_next = {
        let (_, next) = tree.nodes[&to_split].segment_adjacencies_mut();
        let old = *next;
        *next = new_id;
        old
    };

    // adjust the new node
    {
        let (prev, next) = tree.nodes[&new_id].segment_adjacencies_mut();
        *prev = to_split;
        *next = old_to_split_next;
    }

    // adjust the node after `to_split`
    {
        let (prev, _) = tree.nodes[&old_to_split_next].segment_adjacencies_mut();
        *prev = new_id;
    }

    new_id
}

fn lookup_id_index<Id: Hash + Clone + Eq + Debug>(
    tree: &Tree<Id>,
    lookup_id: &Id,
) -> Result<(NodeId, usize), TreeError> {
    let node_id = tree
        .id_to_node
        .get(&lookup_id)
        .ok_or(TreeError::UnknownId)?;
    let node = tree
        .nodes
        .get(&node_id)
        .expect("node_id listed in id_to_node did not exist.");

    let ids = node.segment_ids()?;

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
fn lookup_insertion_point<Id: Hash + Clone + Eq + Debug>(
    tree: &Tree<Id>,
    lookup_id: &Id,
) -> Result<(NodeId, usize, usize), TreeError> {
    let node_id = tree
        .id_to_node
        .get(&lookup_id)
        .ok_or(TreeError::UnknownId)?;
    let node = tree
        .nodes
        .get(&node_id)
        .expect("node_id listed in id_to_node did not exist.");
    if node.segment_is_container() {
        let (_, start) = node.segment_adjacencies();
        return Ok((*start, 0, 0));
    }
    let ids = node.segment_ids()?;

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
        return Ok((*node_id, node.segment_contents_len()?, id_list_index));
    }
    panic!("id not found in segment id list");
}

/// If `segment` is greater than `SPLIT_LEN`, we'll split it into two pieces. This recurses on
/// the children, further splitting them if they're still too long. Returns the leftmost and
/// rightmost of the new segments; if no split occured, these will both still be `segment`.
// TODO this could probably be sped up to instantly segment a very long node into `n` children.
fn consider_split<Id: Hash + Clone + Eq + Debug>(
    tree: &mut Tree<Id>,
    segment: NodeId,
) -> (NodeId, NodeId) {
    if tree.nodes[&segment].segment_is_container() {
        // abort if this is off the edge of a string
        return (segment, segment);
    }
    let ids = tree.nodes[&segment].segment_ids_mut().unwrap();
    if ids.len() <= SPLIT_LEN {
        return (segment, segment);
    }
    // the first index of the second segment. need to do this stuff to make sure we split
    // along a codepoint boundary
    let split_start_vec = ids.len() / 2;
    let new_node_id = insert_segment(tree, segment, split_start_vec);
    let (left, _) = consider_split(tree, segment);
    let (_, right) = consider_split(tree, new_node_id);
    (left, right)
}
