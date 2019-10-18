//! Typed access to a `Tree`.
//!
//! We don't provide mutable parent access because that would allow you to delete something that
//! contained the reference.

use std::fmt::Debug;
use std::hash::Hash;
use super::tree::{Tree};

macro_rules! define_value {
    ($ref_name:ident, $mut_name:ident ref {$($ref_contents:tt)*} mut {$($mut_contents:tt)*}) => {
        pub struct $ref_name<'a, Id: Hash + Clone + Eq + Debug> {
            id: Id,
            tree: &'a Tree<Id>,
        }
        pub struct $mut_name<'a, Id: Hash + Clone + Eq + Debug> {
            id: Id,
            tree: &'a mut Tree<Id>,
        }
        impl <'a, Id: Hash + Clone + Eq + Debug> $ref_name<'a, Id> {
            pub fn parent(&self) -> Option<ParentRef<'a, Id>> {
                let parent = self.tree.get_parent(self.id.clone()).expect("id should have existed if value wrapper existed");
                parent.map(|parent| {
                    match self.tree.get_type(parent.clone()) {
                        Object => {
                            ParentRef::Object(ObjectRef {
                                id: parent,
                                tree: &self.tree,
                            })
                        }
                        // TODO ARRAY
                        _ => panic!("parent was of unexpected type")
                    }
                })
            }
            pub fn id(&self) -> Id {
                self.id.clone()
            }
            $($ref_contents)*
        }
        impl <'a, Id: Hash + Clone + Eq + Debug> $mut_name<'a, Id> {
            pub fn as_ref(&'a self) -> $ref_name<Id> {
                $ref_name {
                    id: self.id.clone(),
                    tree: &self.tree,
                }
            }
            pub fn parent(&'a self) -> Option<ParentRef<'a, Id>> {
                self.as_ref().parent()
            }
            pub fn id(&self) -> Id {
                self.as_ref().id()
            }
            $($ref_contents)*
            $($mut_contents)*
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueType {
    String,
    Character,
    True,
    False,
    Null,
    Object,
    Array,
    ArrayEntry,
}

pub enum ValueRef<'a, Id: Hash + Clone + Eq + Debug> {
    String(StringRef<'a, Id>),
    Bool(BoolRef<'a, Id>),
    Null(NullRef<'a, Id>),
    Object(ObjectRef<'a, Id>),
}
pub enum ValueMut<'a, Id: Hash + Clone + Eq + Debug> {
    String(StringMut<'a, Id>),
    Bool(BoolMut<'a, Id>),
    Null(NullMut<'a, Id>),
    Object(ObjectMut<'a, Id>),
}
pub enum ParentRef<'a, Id: Hash + Clone + Eq + Debug> {
    Object(ObjectRef<'a, Id>),
    // Array(ArrayRef<'a, Id>),
}
pub enum ParentMut<'a, Id: Hash + Clone + Eq + Debug> {
    Object(ObjectMut<'a, Id>),
    // Array(ArrayMut<'a, Id>),
}

define_value!{
    StringRef, StringMut
    ref {
    }
    mut {
    }
}
define_value!{
    ObjectRef, ObjectMut
    ref {
    }
    mut {
    }
}
define_value!{
    BoolRef, BoolMut
    ref {
    }
    mut {
    }
}
define_value!{
    NullRef, NullMut
    ref {
    }
    mut {
    }
}

