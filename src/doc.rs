use crate::json;
use crate::opset;
use std::cmp::Ordering;

const CACHE_GAP: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Id {
    pub num: usize,
}

pub const ROOT_ID: Id = Id { num: 0 };

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocOp {
    pub timestamp: u64,
    pub edits: Vec<json::Edit<Id>>,
}
impl PartialOrd for DocOp {
    fn partial_cmp(&self, other: &DocOp) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for DocOp {
    fn cmp(&self, other: &DocOp) -> Ordering {
        // TODO fix this ordering
        self.timestamp.cmp(&other.timestamp)
    }
}

impl opset::Operation<json::Tree<Id>> for DocOp {
    fn apply(&self, tree: &mut json::Tree<Id>) {
        for edit in &self.edits {
            let _ = tree.update(edit);
        }
    }
}

pub struct Doc {
    opset: opset::Opset<DocOp, json::Tree<Id>>,
}

impl Doc {
    pub fn new() -> Doc {
        Doc {
            opset: opset::Opset::new(json::Tree::new_with_object_root(ROOT_ID), CACHE_GAP),
        }
    }

    pub fn update(&mut self, op: DocOp) {
        self.opset.update(op);
    }

    pub fn update_from_iter<I: std::iter::Iterator<Item = DocOp>>(&mut self, iter: I) {
        self.opset.update_from_iter(iter);
    }

    pub fn tree(&self) -> &json::Tree<Id> {
        self.opset.state()
    }
}
