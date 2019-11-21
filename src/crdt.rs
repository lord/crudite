use crate::tree::Tree;
use std::collections::BTreeMap;

pub trait Edit<State> {
    fn apply(&self, tree: &mut State);
}

pub struct OpSetCrdt<E: Edit<State> + Ord, State: Clone> {
    /// list of all edits applied to this tree
    edits: Vec<E>,
    /// current state of the tree
    state: State,
    /// maps (num edits applied) -> (tree at that point in time)
    old_states: BTreeMap<usize, State>,
}

impl <E: Edit<State> + Ord, State: Clone> OpSetCrdt<E, State> {
    pub fn new(initial_state: State) -> Self {
        OpSetCrdt {
            edits: Vec::new(),
            state: initial_state,
            old_states: BTreeMap::new(),
        }
    }

    pub fn edit(&mut self, edit: E) {
        unimplemented!();
    }

    pub fn state(&self) -> &State {
        &self.state
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
