use crate::tree::Tree;
use std::collections::BTreeMap;

#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub struct Id(u64);

pub enum Op {
    InsertText {
        prev_character: Id,
        text: String,
    },
}

pub struct Edit {
    ops: Vec<Op>,
    site: u64,
    timestamp: u64,
}

pub struct TreeCrdt {
    /// list of all edits applied to this tree
    edits: Vec<Edit>,
    tree: Tree<Id>,
    /// maps (num edits applied) -> (tree at that point in time)
    old_trees: BTreeMap<usize, Tree<Id>>,
}

impl TreeCrdt {
    pub fn new() -> Self {
        TreeCrdt {
            edits: Vec::new(),
            tree: Tree::new_with_object_root(Id(0)),
            old_trees: BTreeMap::new(),
        }
    }

    pub fn edit(&mut self, edit: Edit) {
        unimplemented!();
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
