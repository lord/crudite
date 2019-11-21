use crate::tree::Tree;

pub trait Edit<State> {
    fn apply(&self, tree: &mut State);
}

pub struct OpSetCrdt<E: Edit<S> + Ord, S: Clone> {
    /// list of all edits applied to this tree
    edits: Vec<E>,
    /// a list of (num edits applied, state of tree at that point in time)
    states: Vec<(usize, S)>,
    /// at most, this many edits will be skipped over in the states cache
    cache_gap: usize,
}

impl<E: Edit<S> + Ord, S: Clone> OpSetCrdt<E, S> {
    pub fn new(initial_state: S, cache_gap: usize) -> Self {
        OpSetCrdt {
            edits: Vec::new(),
            cache_gap,
            states: vec![(0, initial_state)],
        }
    }

    pub fn edit(&mut self, edit: E) {
        let insert_point = self
            .edits
            .binary_search(&edit)
            .expect_err("two edits had the same timestamp");
        self.edits.insert(insert_point, edit);
        self.recalculate(insert_point);
    }

    pub fn edit_from_iter<I: std::iter::Iterator<Item = E>>(&mut self, edits: I) {
        let mut least_insert_point = None;
        for edit in edits {
            let insert_point = self
                .edits
                .binary_search(&edit)
                .expect_err("two edits had the same timestamp");
            self.edits.insert(insert_point, edit);
            least_insert_point = match least_insert_point {
                Some(prev) if prev < insert_point => Some(prev),
                _ => Some(insert_point),
            };
        }
        if let Some(least_insert_point) = least_insert_point {
            self.recalculate(least_insert_point);
        }
    }

    /// Recalculates states after the edit list has been changed. The first `insert_point`
    /// edits should be identical to the last time `recalculate` was called.
    fn recalculate(&mut self, insert_point: usize) {
        let index_of_first_bad_state =
            match self.states.binary_search_by_key(&insert_point, |(n, _)| *n) {
                Ok(n) => n + 1,
                Err(n) => n,
            };
        // delete all previous states after least_insert_point, add one so that if something exists
        // exactly at `least_insert_point` it is preserved.
        self.states.truncate(index_of_first_bad_state);
        let (mut applied_edits, mut state) = self.states.pop().unwrap();
        while applied_edits < self.edits.len() {
            if self.states.len() == 0
                || self.states.last().unwrap().0 + self.cache_gap <= applied_edits
            {
                // time to insert a new cache
                self.states.push((applied_edits, state.clone()));
            }
            self.edits[applied_edits].apply(&mut state);
            applied_edits += 1;
        }
        self.states.push((applied_edits, state));
    }

    pub fn state(&self) -> &S {
        &self
            .states
            .last()
            .expect("somehow state cache was empty?")
            .1
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(PartialOrd, Ord, Debug, Clone, Eq, PartialEq)]
    struct TestEdit {
        timestamp: usize,
        value: usize,
    }

    impl Edit<Vec<usize>> for TestEdit {
        fn apply(&self, state: &mut Vec<usize>) {
            state.push(self.value);
        }
    }

    #[test]
    fn various_edits_work() {
        let mut crdt = OpSetCrdt::new(vec![0], 2);
        // initial edit
        crdt.edit(TestEdit {
            timestamp: 10,
            value: 1,
        });
        assert_eq!(crdt.state(), &[0, 1]);
        assert_eq!(crdt.states.len(), 2);

        // edit before start
        crdt.edit(TestEdit {
            timestamp: 5,
            value: 2,
        });
        assert_eq!(crdt.state(), &[0, 2, 1]);
        assert_eq!(crdt.states.len(), 2);

        // edit at end
        crdt.edit(TestEdit {
            timestamp: 15,
            value: 3,
        });
        assert_eq!(crdt.state(), &[0, 2, 1, 3]);
        assert_eq!(crdt.states.len(), 3);

        // edit in middle
        crdt.edit(TestEdit {
            timestamp: 12,
            value: 4,
        });
        assert_eq!(crdt.state(), &[0, 2, 1, 4, 3]);
        assert_eq!(crdt.states.len(), 3);

        // one more edit
        crdt.edit(TestEdit {
            timestamp: 11,
            value: 5,
        });
        assert_eq!(crdt.state(), &[0, 2, 1, 5, 4, 3]);
        assert_eq!(crdt.states.len(), 4);
    }

    #[test]
    fn various_edits_work_with_iter() {
        let mut crdt = OpSetCrdt::new(vec![0], 2);

        let edits = vec![
            TestEdit {
                timestamp: 10,
                value: 1,
            },
            TestEdit {
                timestamp: 5,
                value: 2,
            },
            TestEdit {
                timestamp: 15,
                value: 3,
            },
            TestEdit {
                timestamp: 12,
                value: 4,
            },
            TestEdit {
                timestamp: 11,
                value: 5,
            },
        ];
        crdt.edit_from_iter(edits.into_iter());
        assert_eq!(crdt.state(), &[0, 2, 1, 5, 4, 3]);
        assert_eq!(crdt.states.len(), 4);
    }
}
