use std::cmp::Ordering;
use crate::opset;
use crate::tree;

const CACHE_GAP: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Id {
    pub num: usize,
}

pub const ROOT_ID: Id = Id {
    num: 0,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocOp {
    pub timestamp: u64,
    pub edits: Vec<tree::Edit<Id>>,
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

impl opset::Operation<tree::Tree<Id>> for DocOp {
    fn apply(&self, tree: &mut tree::Tree<Id>) {
        for edit in &self.edits {
            let _ = tree.update(edit);
        }
    }
}

pub struct Doc {
    opset: opset::Opset<DocOp, tree::Tree<Id>>,
}

impl Doc {
    pub fn new() -> Doc {
        Doc {
            opset: opset::Opset::new(tree::Tree::new(ROOT_ID), CACHE_GAP),
        }
    }

    pub fn update(&mut self, op: DocOp) {
        self.opset.update(op);
    }

    pub fn update_from_iter<I: std::iter::Iterator<Item = DocOp>>(&mut self, iter: I) {
        self.opset.update_from_iter(iter);
    }
}
