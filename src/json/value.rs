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
pub enum Parent<Id> {
    Array(ArrayRef<Id>),
    Object(ObjectRef<Id>),
    None,
}

pub(super) fn get_parent<Id: Hash + Clone + Eq + Debug>(
    tree: &tree::Tree<Id>,
    id: &Id,
) -> Result<Parent<Id>, tree::TreeError> {
    let id = match tree.get_parent(id.clone())? {
        Some(v) => v,
        None => return Ok(Parent::None),
    };
    match tree.get_type(id.clone()) {
        Ok(tree::NodeType::Array) => Ok(Parent::Array(ArrayRef(id))),
        Ok(tree::NodeType::Object) => Ok(Parent::Object(ObjectRef(id))),
        e => panic!("parent was of unexpected type: {:?}", e),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringRef<Id>(pub Id);
impl<Id: Hash + Clone + Eq + Debug> StringRef<Id> {
    pub fn to_string(&self, tree: &tree::Tree<Id>) -> Result<String, tree::TreeError> {
        let string_node_id = tree
            .id_to_node(&self.0)?;
        let node = tree
            .nodes
            .get(&string_node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let mut next = match &node.data {
            tree::NodeData::String { start, .. } => *start,
            _ => return Err(tree::TreeError::UnexpectedNodeType),
        };
        let mut string = String::new();
        while next != string_node_id {
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

    pub fn start(&self, _tree: &tree::Tree<Id>) -> Result<StringIndex<Id>, tree::TreeError> {
        // TODO VALIDATE STILL IN TREE HERE
        Ok(StringIndex(self.0.clone()))
    }

    pub fn end(&self, _tree: &tree::Tree<Id>) -> Result<StringIndex<Id>, tree::TreeError> {
        unimplemented!()
    }

    pub fn parent(&self, tree: &tree::Tree<Id>) -> Result<Parent<Id>, tree::TreeError> {
        get_parent(&tree, &self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringIndex<Id>(pub Id);
impl<Id: Hash + Clone + Eq + Debug> StringIndex<Id> {
    pub fn parent(&self, tree: &tree::Tree<Id>) -> Result<StringRef<Id>, tree::TreeError> {
        match tree.get_type(self.0.clone()) {
            Ok(tree::NodeType::String) => Ok(StringRef(self.0.clone())),
            Ok(tree::NodeType::Character) => Ok(StringRef(
                tree.get_parent(self.0.clone())?
                    .expect("Stringsegment should have parent"),
            )),
            Ok(_) => Err(tree::TreeError::UnexpectedNodeType),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayRef<Id>(pub Id);
impl<Id: Hash + Clone + Eq + Debug> ArrayRef<Id> {
    pub fn to_vec(&self, tree: &tree::Tree<Id>) -> Result<Vec<Value<Id>>, tree::TreeError> {
        let string_node_id = tree
            .id_to_node(&self.0)?;
        let node = tree
            .nodes
            .get(&string_node_id)
            .expect("node_id listed in id_to_node did not exist.");
        let mut next = match &node.data {
            tree::NodeData::Array { start, .. } => *start,
            _ => return Err(tree::TreeError::UnexpectedNodeType),
        };
        let mut children = Vec::new();
        while next != string_node_id {
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
        let values = children
            .iter()
            .map(|child| tree.child_to_value(Some(child)))
            .collect();
        Ok(values)
    }

    pub fn parent(&self, tree: &tree::Tree<Id>) -> Result<Parent<Id>, tree::TreeError> {
        get_parent(&tree, &self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayIndex<Id>(pub Id);
impl<Id: Hash + Clone + Eq + Debug> ArrayIndex<Id> {
    pub fn parent(&self, tree: &tree::Tree<Id>) -> Result<ArrayRef<Id>, tree::TreeError> {
        match tree.get_type(self.0.clone()) {
            Ok(tree::NodeType::Array) => Ok(ArrayRef(self.0.clone())),
            Ok(tree::NodeType::ArrayEntry) => Ok(ArrayRef(
                tree.get_parent(self.0.clone())?
                    .expect("arraysegment should have parent"),
            )),
            Ok(_) => Err(tree::TreeError::UnexpectedNodeType),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRef<Id>(pub Id);
impl<Id: Hash + Clone + Eq + Debug> ObjectRef<Id> {
    pub fn parent(&self, tree: &tree::Tree<Id>) -> Result<Parent<Id>, tree::TreeError> {
        get_parent(&tree, &self.0)
    }

    pub fn get(&self, tree: &tree::Tree<Id>, key: &str) -> Result<Value<Id>, tree::TreeError> {
        let object_node_id = tree
            .id_to_node(&self.0)?;
        let child = match &tree.nodes[&object_node_id].data {
            tree::NodeData::Object { items, id: _ } => items.get(key),
            _ => return Err(tree::TreeError::UnexpectedNodeType),
        };
        Ok(tree.child_to_value(child))
    }
}
