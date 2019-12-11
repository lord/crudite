use im::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use super::tree;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value<Id> {
    String(StringRef<Id>),
    Array(ArrayRef<Id>),
    Object(ObjectRef<Id>),
    Int(i64),
    True,
    False,
    Null,
    Unset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringRef<Id>(pub Id);
impl <Id: Hash + Clone + Eq + Debug> StringRef<Id> {
    pub fn to_string(&self, tree: &tree::Tree<Id>) -> Result<String, tree::TreeError> {
        let string_node_id = tree
            .id_to_node
            .get(&self.0)
            .ok_or(tree::TreeError::UnknownId)?;
        let node = tree
            .nodes
            .get(&string_node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let mut next = match &node.data {
            tree::NodeData::String { start, .. } => *start,
            _ => return Err(tree::TreeError::UnexpectedNodeType),
        };
        let mut string = String::new();
        while next != *string_node_id {
            let node = tree
                .nodes
                .get(&next)
                .expect("node_id listed in segment adjacency did not exist.");
            next = match &node.data {
                tree::NodeData::StringSegment { next, contents, .. } => {
                    string.push_str(contents);
                    *next
                }
                _ => panic!("debug_get_string called on non-string Id"),
            };
        }
        Ok(string)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayRef<Id>(pub Id);
impl <Id: Hash + Clone + Eq + Debug> ArrayRef<Id> {
    pub fn to_vec(&self, tree: &tree::Tree<Id>) -> Result<Vec<Value<Id>>, tree::TreeError> {
        let string_node_id = tree
            .id_to_node
            .get(&self.0)
            .ok_or(tree::TreeError::UnknownId)?;
        let node = tree
            .nodes
            .get(&string_node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let mut next = match &node.data {
            tree::NodeData::Array { start, .. } => *start,
            _ => return Err(tree::TreeError::UnexpectedNodeType),
        };
        let mut children = Vec::new();
        while next != *string_node_id {
            let node = tree
                .nodes
                .get(&next)
                .expect("node_id listed in segment adjacency did not exist.");
            next = match &node.data {
                tree::NodeData::ArraySegment { next, contents, .. } => {
                    children.extend(contents.iter());
                    *next
                }
                _ => panic!("debug_get_string called on non-string Id"),
            };
        }
        let values = children.iter().map(|child| tree.child_to_value(Some(child))).collect();
        Ok(values)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRef<Id>(pub Id);
impl <Id: Hash + Clone + Eq + Debug> ObjectRef<Id> {
}
