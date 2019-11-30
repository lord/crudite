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
- [ ] add array type to json tree
- [ ] bad IDs are currently ignored by `DocOp`'s `apply`. is this right? how can we prevent malicious reuse of ids? central server validation?

## future work

- [ ] garbage collection
- [ ] selective subtree sync
- [ ] splice operations?
- [ ] maybe edits aren't actually ord, figure out id system for edits so we can delete them as well? will allow us to have floats also
- [ ] figure out true cost of all the tree deleting. can we speed up or defer the deletions when an old edit is inserted early in the oplist, or when object_assign deletes a large subtree?
