use std::cmp::Ordering;
use crate::opset;
use crate::tree::Tree;

const CACHE_GAP: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Id {
    num: usize,
}

const ROOT_ID: Id = Id {
    num: 0,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    True,
    False,
    // TODO number
    Null,
    Collection(Id),
    Undefined,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    // MakeList {
    //     /// id of new list
    //     obj: Id,
    // },
    MapCreate {
        /// id of new map
        obj: Id,
    },
    TextCreate {
        /// id of new text
        obj: Id,
    },
    // ListInsert {
    //     /// Id of list to insert into.
    //     parent: Id,
    //     /// If new item is at start of list, `prev` is `None`.
    //     prev: Option<Id>,
    //     /// Item to be inserted. If this item had a prevous parent, it is removed from that parent.
    //     obj: Value,
    // },
    MapInsert {
        /// Id of hashmap to insert into.
        parent: Id,
        /// Key of item in hashmap
        key: String,
        /// Item to be set. If this item had a prevous parent, it is removed from that parent.
        obj: Value,
    },
    TextInsert {
        /// Id of list to insert into.
        parent: Id,
        /// If new item is at start of list, `prev` is `None`.
        prev: Option<Id>,
        /// Id of newly created character
        obj: Id,
        /// Actual new character value
        character: char,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocOp {
    timestamp: u64,
    edits: Vec<Edit>,
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

impl opset::Operation<Tree<Id>> for DocOp {
    fn apply(&self, tree: &mut Tree<Id>) {
        unimplemented!()
    }
}

pub struct Doc {
    opset: opset::Opset<DocOp, Tree<Id>>,
}

impl Doc {
    pub fn new() -> Doc {
        Doc {
            opset: opset::Opset::new(Tree::new(ROOT_ID), CACHE_GAP),
        }
    }

    pub fn insert(&mut self, op: DocOp) {
        self.opset.insert(op);
    }

    pub fn insert_from_iter<I: std::iter::Iterator<Item = DocOp>>(&mut self, iter: I) {
        self.opset.insert_from_iter(iter);
    }
}
