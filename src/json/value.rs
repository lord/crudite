//! Typed access to a `Tree`.
//!
//! We don't provide mutable parent access because that would allow you to delete something that
//! contained the reference.

use super::tree::Tree;
use std::fmt::Debug;
use std::hash::Hash;

macro_rules! define_value {
    ($ref_name:ident, $mut_name:ident ref {$($ref_contents:tt)*} mut {$($mut_contents:tt)*}) => {
        define_value!($ref_name ref {$($ref_contents)*});
        pub struct $mut_name<'a, Id: Hash + Clone + Eq + Debug> {
            id: Id,
            tree: &'a mut Tree<Id>,
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
    };

    ($ref_name:ident ref {$($ref_contents:tt)*}) => {
        pub struct $ref_name<'a, Id: Hash + Clone + Eq + Debug> {
            id: Id,
            tree: &'a Tree<Id>,
        }
        impl <'a, Id: Hash + Clone + Eq + Debug> $ref_name<'a, Id> {
            pub fn parent(&self) -> Option<ParentRef<'a, Id>> {
                let parent = self.tree.get_parent(self.id.clone()).expect("id should have existed if value wrapper existed");
                parent.map(|parent| {
                    match self.tree.get_type(parent.clone()) {
                        Ok(ValueType::Object) => {
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
    True(TrueRef<'a, Id>),
    False(FalseRef<'a, Id>),
    Null(NullRef<'a, Id>),
    Object(ObjectRef<'a, Id>),
}
pub enum ValueMut<'a, Id: Hash + Clone + Eq + Debug> {
    String(StringMut<'a, Id>),
    True(TrueRef<'a, Id>),
    False(FalseRef<'a, Id>),
    Null(NullRef<'a, Id>),
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

pub struct StringRefIter<'a, Id: Hash + Clone + Eq + Debug> {
    inner: StringRef<'a, Id>,
}
impl <'a, Id: Hash + Clone + Eq + Debug> StringRefIter<'a, Id> {
    fn jump_to(&mut self, id: Id) -> Result<(), ()> {
        unimplemented!()
    }
    pub fn set_direction(&mut self, forward: bool) {
        unimplemented!()
    }
}
impl <'a, Id: Hash + Clone + Eq + Debug> std::iter::Iterator for StringRefIter<'a, Id> {
    type Item = (Id, char);
    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

define_value! {
    StringRef, StringMut
    ref {
        pub fn to_string(&self) -> String {
            unimplemented!()
        }
        pub fn start(&self) -> Id {
            unimplemented!()
        }
        pub fn end(&self) -> Id {
            unimplemented!()
        }
        pub fn iter_chars(&self) -> StringRefIter<'a, Id> {
            unimplemented!()
        }
    }
    mut {
        pub fn insert_str(&mut self, insert_point: Id, string: &str) {
            unimplemented!()
        }
        pub fn delete_char(&mut self, start: Id, end: Id) {
            unimplemented!()
        }
        pub fn delete_range(&mut self, start: Id, end: Id) {
            unimplemented!()
        }
    }
}
define_value! {
    ObjectRef, ObjectMut
    ref {
        pub fn get(&self, key: &str) -> Option<ValueRef<Id>> {
            unimplemented!()
        }
    }
    mut {
        pub fn get_mut(&mut self, key: &str) -> Option<ValueMut<Id>> {
            unimplemented!()
        }
    }
}
define_value! {
    TrueRef
    ref {
    }
}
define_value! {
    FalseRef
    ref {
    }
}
define_value! {
    NullRef
    ref {
    }
}
