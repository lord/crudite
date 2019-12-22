# crudite

CRDT library.

## Requirements

- performant even on massive data structures
- allows syncing just part of a massive data structure
- allows arbitrary data structures, with a structured tree-like representation
- has a way of representing, inserting, and splicing text
- bring-your-own network code, but CRDT algorithm does not require a central server

## notes

starting with simple opset crdt, no splicing


## todo

- [x] implement operation linearization
- [x] add CRDT tests
- [x] actual op/edit/crdt struct that combines opsets and jsontree
- [x] add number type to json tree
- [x] finish update fn for Edit
- [x] cleanup code before arrays
  - [x] test for character deletion, make sure segments merge
  - [x] check case where segment can't merge but reaches zero length
  - [x] move segment code into separate module, test segments properly
  - [x] don't actually need to delete segments until we have garbage collection
- [x] add array type to json tree
- [x] moving objects needs some work; need to check for cycles, need to remove item from previous parent. maybe can combine this fix with the orphaning system.
- [x] update tests to actually test all those new parent() fns
- [x] finish upgrading tests to public Value APIs, add more Value methods, esp. for ergonomic string/list index access
- [ ] system for constructing a StringRef or ObjectRef or ArrayRef in the first place
- [ ] need to keep track of parents; you shouldn't need to delete to re-add, this makes it so multiple moves from the same place won't compose correctly.

## future work

- [ ] bad IDs are currently ignored by `DocOp`'s `apply`. is this right? how can we prevent malicious reuse of ids? central server validation?
- [ ] fuzz for panics and other bugs
- [ ] garbage collection
- [ ] selective subtree sync
- [ ] splice operations?
- [ ] maybe edits aren't actually ord, figure out id system for edits so we can delete them as well? will allow us to have floats also
- [ ] figure out true cost of all the tree deleting. can we speed up or defer the deletions when an old edit is inserted early in the oplist, or when object_assign deletes a large subtree?

## gc notes

- we can also 'freeze' document state at a particular point in time. this is called a 'baseline'. some user-based crdt folks think this is unnecessary bc user-edited data isn't large. but it'd be nice to have an entire database that is a crdt. this could be way too many edits. synchronizing entire histories isn't possible.
- considering we can serialize the state of the document at a particular point in the operation tree, i think we can pass to a new client the document state as it existed at baseline T and only have to send edits that have occured since T in the timeline.
- however due to the asynchronous nature of offline mode, we can't ensure all clients have seen T. so what can we do? i'm comfortable with having a central server of some kind keeping track of all edits, not just the garbage collected ones since T occurred. perhaps it's possible this central server can operational-transform modify the edits to happen later? unfortunately this runs into the classic OT problem of `O(N^2)` performance, where `N` is the number of changes in each distinct site. the opset CRDT algorithm lets us insert characters very early in the algorithm and can merge in ~`O(N)`, assuming constant time operation processing.
- perhaps we can update a site's baseline. the central server pays the `O(N)` cost to run the new edits up to baseline `T`, to form a new state `T'`. it then diffs `T` and `T'` and sends the diffs to any site that is using `T`, which applies the changes to their baseline. is diffing an expensive operation? can it be made cheaper? we could keep track of anything that was edited at all while running the new edits up to the baseline, consider those dirty, and just diff those?
