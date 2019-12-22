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
        let string_node_id = tree.id_to_node(&self.0)?;
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

    pub fn start(&self, tree: &tree::Tree<Id>) -> Result<StringIndex<Id>, tree::TreeError> {
        // validate still in tree
        let _ = tree.id_to_node(&self.0)?;
        Ok(StringIndex(self.0.clone()))
    }

    pub fn end(&self, tree: &tree::Tree<Id>) -> Result<StringIndex<Id>, tree::TreeError> {
        let node_id = tree.id_to_node(&self.0)?;
        let node = tree.nodes.get(&node_id).expect("node_id listed in id_to_node did not exist.");
        let last_node_id = match &node.data {
            tree::NodeData::String { end, .. } => *end,
            _ => return Err(tree::TreeError::UnexpectedNodeType),
        };
        let last_node = tree.nodes.get(&last_node_id).unwrap();
        match &last_node.data {
            tree::NodeData::StringSegment { ids, .. } => {
                Ok(StringIndex(ids.last().unwrap().0.clone()))
            },
            _ => Err(tree::TreeError::UnexpectedNodeType),
        }
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

    fn adjacent_next(&self, tree: &tree::Tree<Id>, backwards: bool) -> Result<StringIndex<Id>, tree::TreeError> {
        let node_id = tree.id_to_node(&self.0)?;
        let mut this_node = tree.nodes.get(&node_id).expect("node_id listed in id_to_node did not exist.");
        let mut this_index = match &this_node.data {
            tree::NodeData::StringSegment { ids, .. } => {
                let pos = ids.iter().position(|(id, _)| id == &self.0).unwrap();
                pos
            },
            tree::NodeData::String { start, .. } => {
                0
            }
            _ => return Err(tree::TreeError::UnexpectedNodeType),
        };

        loop {
            let (next_node, next_index) = match &this_node.data {
                tree::NodeData::String { start, .. } => {
                    if backwards {
                        // started at start of string and going backwards; return self
                        return Ok(self.clone())
                    }
                    (tree.nodes.get(&start).unwrap(), None)
                },
                tree::NodeData::StringSegment { ids, next, prev, .. } => {
                    if (backwards && this_index > 0) || (!backwards && this_index+1 < ids.len()) {
                        (this_node, Some(if backwards {this_index-1} else {this_index+1}))
                    } else {
                        if backwards {
                            (tree.nodes.get(&prev).unwrap(), None)
                        } else {
                            (tree.nodes.get(&next).unwrap(), None)
                        }
                    }
                },
                _ => return Err(tree::TreeError::UnexpectedNodeType),
            };

            match (&this_node.data, &next_node.data) {
                (tree::NodeData::String { .. }, tree::NodeData::String { .. }) => {
                    panic!("invalid string data structure")
                }
                (tree::NodeData::StringSegment { ids, .. }, tree::NodeData::String { id, .. }) => {
                    // hit edge of string; return
                    if backwards {
                        return Ok(StringIndex(id.clone()))
                    } else {
                        return Ok(StringIndex(ids[this_index].0.clone()))
                    }
                }
                (_, tree::NodeData::StringSegment { ids, .. }) => {
                    this_node = next_node;
                    this_index = next_index.unwrap_or(if backwards && ids.len() > 0 {ids.len()-1} else {0});
                    if ids.len() > 0 {
                        if ids[this_index].1.is_some() {
                            return Ok(StringIndex(ids[this_index].0.clone()))
                        }
                    }
                }
                _ => panic!("invalid node types")
            }
        }
    }

    pub fn still_exists(&self, tree: &tree::Tree<Id>) -> bool {
        let node_id = match tree.id_to_node(&self.0) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let this_node = tree.nodes.get(&node_id).unwrap();
        match &this_node.data {
            tree::NodeData::StringSegment { ids, .. } => {
                let pos = ids.iter().position(|(id, _)| id == &self.0).unwrap();
                ids[pos].1.is_some()
            },
            tree::NodeData::String { start, .. } => {
                true
            }
            _ => false,
        }
    }

    /// Returns the index that is `num` characters away from `self`. If reaches start or end of
    /// string, will stop. Takes `O(n)`; make take longer if there are a lot of deleted characters
    /// to traverse over.
    pub fn adjacent(&self, tree: &tree::Tree<Id>, mut num: i64) -> Result<StringIndex<Id>, tree::TreeError> {
        {
            if num > 0 && !self.still_exists(tree) {
                // character doesn't exist and we're moving forward; so add one to number so that
                // we resolve the character to its true position
                num += 1;
            }
        }

        let mut i = self.clone();

        while num != 0 {
            if num > 0 {
                i = i.adjacent_next(tree, false)?;
                num -= 1;
            } else {
                i = i.adjacent_next(tree, true)?;
                num += 1;
            }
        }

        Ok(i)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayRef<Id>(pub Id);
impl<Id: Hash + Clone + Eq + Debug> ArrayRef<Id> {
    pub fn to_vec(&self, tree: &tree::Tree<Id>) -> Result<Vec<Value<Id>>, tree::TreeError> {
        let string_node_id = tree.id_to_node(&self.0)?;
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
        let object_node_id = tree.id_to_node(&self.0)?;
        let child = match &tree.nodes[&object_node_id].data {
            tree::NodeData::Object { items, id: _ } => items.get(key),
            _ => return Err(tree::TreeError::UnexpectedNodeType),
        };
        Ok(tree.child_to_value(child))
    }
}
