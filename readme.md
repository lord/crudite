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

- [ ] implement rest of value
- [ ] implement CRDT timestamps and operation linearization
- [ ] add CRDT tests
- [ ] add number type to json tree
- [ ] add array type to json tree
- [ ] splice operations?