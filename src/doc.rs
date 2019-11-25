use std::cmp::Ordering;
use crate::opset;
use crate::tree::Tree;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Id {
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    True,
    False,
    // TODO number
    Null,
    Collection(Id),
    Undefined,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Edit {
    MapCreate {
        /// id of new map
        obj: Id,
    },
    // MakeList {
    //     /// id of new list
    //     obj: Id,
    // },
    TextCreate {
        /// id of new text
        obj: Id,
    },
    TextInsert {
        /// Id of list to insert into.
        parent: Id,
        /// If new item is at start of list, `prev` is `None`.
        key: Option<Id>,
        /// Id of newly created character
        obj: Value,
        /// Actual new character value
        character: char,
    },
    // ListInsert {
    //     /// Id of list to insert into.
    //     parent: Id,
    //     /// If new item is at start of list, `prev` is `None`.
    //     key: Option<Id>,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DocOp {
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

struct Doc {
    opset: opset::Opset<DocOp, Tree<Id>>,
}

impl Doc {
}
