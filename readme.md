# crudite

CRDT library.

## Requirements

- performant even on massive data structures
- allows syncing just part of a massive data structure
- allows arbitrary data structures, with a structured tree-like representation
- has a way of representing, inserting, and splicing text
- bring-your-own network code, but CRDT algorithm does not require a central server

## notes

- i think maybe garbage collection is a similar problem to the rebasing thing. if it's possible to have 'dead' subtrees that we don't synchronize??

create_node
set_parent


NEED OPERATIONS THAT ARE APPEND ONLY AND CAN ARRIVE IN ANY ORDER

nodes: Vec<Uid>,
parent_sets: Vec<Uid, Uid>,

-> problem with this model is that we can't delete things, since we can't reparent 'all' their children if we don't know who their children are

## another approach

- assume that we have a CRDT that has a 'splice' operation for text
  -> deleting can just be a splice operation
  -> inserting is basically a splice operation but the source is "new" instead of an existing thing
  -> everything is splicing. you just need to figure out what the splice operation is


## ok ok ok

nodes: Vec<Uid>,
merges: HashMap<Uid, (Uid, Timestamp)>,