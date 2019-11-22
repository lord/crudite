use std::cmp::Ordering;
use crate::opset;
use crate::tree::Tree;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Id {
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Edit {
    timestamp: u64,
}
impl PartialOrd for Edit {
    fn partial_cmp(&self, other: &Edit) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Edit {
    fn cmp(&self, other: &Edit) -> Ordering {
        // TODO fix this ordering
        self.timestamp.cmp(&other.timestamp)
    }
}

impl opset::Operation<Tree<Id>> for Edit {
    fn apply(&self, tree: &mut Tree<Id>) {
        unimplemented!()
    }
}

struct Document {
    opset: opset::Opset<Edit, Tree<Id>>,
}

impl Document {
}
