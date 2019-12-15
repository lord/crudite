use super::tree::*;
use super::value::{self, Value};
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct MyId(usize);

fn vals_to_nums<Id>(vals: Vec<Value<Id>>) -> Vec<i64> {
    vals.iter()
        .map(|val| match val {
            Value::Int(i) => *i,
            _ => panic!("unexpected type in list"),
        })
        .collect()
}

#[test]
fn object_assignment() {
    let mut tree = Tree::new_with_object_root(MyId(0));

    // {}
    // ^
    // 0
    assert_eq!(
        Ok(value::Parent::None),
        value::ObjectRef(MyId(0)).parent(&tree)
    );
    assert_eq!(Ok(NodeType::Object), tree.get_type(MyId(0)));

    tree.construct_object(MyId(1)).unwrap();
    tree.object_assign(
        MyId(0),
        "my key".to_string(),
        Value::Object(value::ObjectRef(MyId(1))),
    )
    .unwrap();

    tree.construct_string(MyId(2)).unwrap();
    tree.object_assign(
        MyId(1),
        "my key 2".to_string(),
        Value::Object(value::ObjectRef(MyId(2))),
    )
    .unwrap();

    tree.insert_character(MyId(2), MyId(3), 'a').unwrap();

    tree.delete_orphans();

    // {"my key": {"my key 2": "a"}}
    // ^          ^            ^^
    // 0          1            23
    assert_eq!(Ok(NodeType::Object), tree.get_type(MyId(0)));
    assert_eq!(Ok(NodeType::Object), tree.get_type(MyId(1)));
    assert_eq!(Ok(NodeType::String), tree.get_type(MyId(2)));
    assert_eq!(Ok(NodeType::Character), tree.get_type(MyId(3)));
    assert_eq!(
        Ok(value::Parent::Object(value::ObjectRef(MyId(0)))),
        value::ObjectRef(MyId(1)).parent(&tree)
    );
    assert_eq!(
        Ok(value::Parent::Object(value::ObjectRef(MyId(1)))),
        value::ObjectRef(MyId(2)).parent(&tree)
    );
    assert_eq!(
        Ok(value::StringRef(MyId(2))),
        value::StringIndex(MyId(3)).parent(&tree)
    );
    assert_eq!(
        Ok(Value::Object(value::ObjectRef(MyId(1)))),
        value::ObjectRef(MyId(0)).get(&tree, "my key")
    );
    assert_eq!(
        Ok(Value::String(value::StringRef(MyId(2)))),
        value::ObjectRef(MyId(1)).get(&tree, "my key 2")
    );
    assert_eq!(Ok(Value::Unset), value::ObjectRef(MyId(0)).get(&tree, "my key 2"));

    tree.object_assign(MyId(0), "my key".to_string(), Value::True)
        .unwrap();

    tree.delete_orphans();

    // {"my key": true}
    // ^          ^
    // 0          4
    assert_eq!(Ok(NodeType::Object), tree.get_type(MyId(0)));
    assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(1)));
    assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(2)));
    assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(3)));
    assert_eq!(
        Ok(value::Parent::None),
        value::ObjectRef(MyId(0)).parent(&tree)
    );
    assert_eq!(
        Err(TreeError::UnknownId),
        value::ObjectRef(MyId(1)).parent(&tree)
    );
    assert_eq!(
        Err(TreeError::UnknownId),
        value::ObjectRef(MyId(2)).parent(&tree)
    );
    assert_eq!(
        Err(TreeError::UnknownId),
        value::StringIndex(MyId(3)).parent(&tree)
    );

    assert_eq!(Ok(Value::True), value::ObjectRef(MyId(0)).get(&tree, "my key"));

    tree.object_assign(MyId(0), "my key".to_string(), Value::Unset)
        .unwrap();

    tree.delete_orphans();

    // {}
    // ^
    // 0
    assert_eq!(Ok(NodeType::Object), tree.get_type(MyId(0)));
    assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(1)));
    assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(2)));
    assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(3)));
    assert_eq!(Err(TreeError::UnknownId), tree.get_type(MyId(4)));
    assert_eq!(Ok(Value::Unset), value::ObjectRef(MyId(0)).get(&tree, "my key"));
}

#[test]
fn invalid_ids_error() {
    let mut tree = Tree::new_with_string_root(MyId(0));
    assert_eq!(tree.insert_character(MyId(0), MyId(1), 'a'), Ok(()));
    assert_eq!(
        tree.insert_character(MyId(0), MyId(1), 'a'),
        Err(TreeError::DuplicateId)
    );
    assert_eq!(
        tree.insert_character(MyId(1), MyId(0), 'a'),
        Err(TreeError::DuplicateId)
    );
    assert_eq!(
        tree.insert_character(MyId(2), MyId(5), 'a'),
        Err(TreeError::UnknownId)
    );
    assert_eq!(tree.delete_character(MyId(2)), Err(TreeError::UnknownId));
    assert_eq!(
        tree.delete_character(MyId(0)),
        Err(TreeError::UnexpectedNodeType)
    );
}

#[test]
fn simple_delete() {
    let mut tree = Tree::new_with_string_root(MyId(0));
    tree.insert_character(MyId(0), MyId(1), 'a').unwrap();
    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok("a".to_string()));
    tree.insert_character(MyId(1), MyId(2), 'b').unwrap();
    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok("ab".to_string()));
    tree.delete_character(MyId(1)).unwrap();
    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok("b".to_string()));
    // test delete same char; should be noop
    tree.delete_character(MyId(1)).unwrap();
    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok("b".to_string()));
    tree.delete_character(MyId(2)).unwrap();
    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok("".to_string()));
}

#[test]
fn insert_and_delete_characters() {
    fn num_to_char(i: usize) -> char {
        match i % 5 {
            0 => '0',
            1 => '1',
            2 => '2',
            3 => '3',
            _ => '4',
        }
    }

    let mut tree = Tree::new_with_string_root(MyId(0));
    tree.insert_character(MyId(0), MyId(1), 'a').unwrap();
    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok("a".to_string()));
    tree.insert_character(MyId(1), MyId(2), 'b').unwrap();
    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok("ab".to_string()));
    tree.insert_character(MyId(1), MyId(3), 'c').unwrap();
    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok("acb".to_string()));
    tree.insert_character(MyId(0), MyId(4), 'd').unwrap();
    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok("dacb".to_string()));
    for i in 5..10000 {
        tree.insert_character(MyId(i - 1), MyId(i), num_to_char(i))
            .unwrap();
    }

    let long_insert = (5..10000).map(|i| num_to_char(i)).collect::<String>();
    assert_eq!(
        value::StringRef(MyId(0)).to_string(&tree),
        Ok(format!("d{}acb", long_insert))
    );

    for i in 5..10000 {
        tree.delete_character(MyId(i)).unwrap();
    }

    assert_eq!(value::StringRef(MyId(0)).to_string(&tree), Ok(format!("dacb")));
}

#[test]
fn insert_and_delete_list_of_nums() {
    fn num_to_value(i: usize) -> Value<MyId> {
        Value::Int(i as i64)
    }

    let mut tree = Tree::new_with_array_root(MyId(0));
    tree.insert_list_item(MyId(0), MyId(1), Value::Int(1))
        .unwrap();
    assert_eq!(value::ArrayRef(MyId(0)).to_vec(&tree).map(vals_to_nums), Ok(vec![1]));
    tree.insert_list_item(MyId(1), MyId(2), Value::Int(2))
        .unwrap();
    assert_eq!(value::ArrayRef(MyId(0)).to_vec(&tree).map(vals_to_nums), Ok(vec![1, 2]));
    tree.insert_list_item(MyId(1), MyId(3), Value::Int(3))
        .unwrap();
    assert_eq!(value::ArrayRef(MyId(0)).to_vec(&tree).map(vals_to_nums), Ok(vec![1, 3, 2]));
    tree.insert_list_item(MyId(0), MyId(4), Value::Int(4))
        .unwrap();
    assert_eq!(value::ArrayRef(MyId(0)).to_vec(&tree).map(vals_to_nums), Ok(vec![4, 1, 3, 2]));
    for i in 5..10000 {
        tree.insert_list_item(MyId(i - 1), MyId(i), num_to_value(i))
            .unwrap();
    }

    let mut long_insert = (5..10000).map(|i| i).collect::<Vec<_>>();
    long_insert.insert(0, 4);
    long_insert.push(1);
    long_insert.push(3);
    long_insert.push(2);
    assert_eq!(value::ArrayRef(MyId(0)).to_vec(&tree).map(vals_to_nums), Ok(long_insert));

    for i in 5..10000 {
        tree.delete_list_item(MyId(i)).unwrap();
    }

    assert_eq!(value::ArrayRef(MyId(0)).to_vec(&tree).map(vals_to_nums), Ok(vec![4, 1, 3, 2]));
}

#[test]
fn cant_move_things_with_object_parents() {
    let mut tree = Tree::new_with_object_root(MyId(0));
    tree.construct_object(MyId(1)).unwrap();
    tree.object_assign(
        MyId(0),
        "my key".to_string(),
        Value::Object(value::ObjectRef(MyId(1))),
    )
    .unwrap();
    // attempt second assignment
    assert_eq!(
        Err(TreeError::NodeAlreadyHadParent),
        tree.object_assign(
            MyId(0),
            "my key 2".to_string(),
            Value::Object(value::ObjectRef(MyId(1)))
        )
    );
}

#[test]
fn cant_move_things_with_array_parents() {
    let mut tree = Tree::new_with_array_root(MyId(0));
    tree.construct_object(MyId(1)).unwrap();
    tree.insert_list_item(MyId(0), MyId(2), Value::Object(value::ObjectRef(MyId(1))))
        .unwrap();
    // attempt second insert
    assert_eq!(
        Err(TreeError::NodeAlreadyHadParent),
        tree.insert_list_item(MyId(0), MyId(3), Value::Object(value::ObjectRef(MyId(1))))
    );
}

#[test]
fn object_assignment_prevents_cycles() {
    let mut tree = Tree::new_with_object_root(MyId(0));

    // {}
    // ^
    // 0
    assert_eq!(Ok(NodeType::Object), tree.get_type(MyId(0)));
    assert_eq!(
        Ok(value::Parent::None),
        value::ObjectRef(MyId(0)).parent(&tree)
    );

    tree.construct_object(MyId(1)).unwrap();
    tree.object_assign(
        MyId(0),
        "my key".to_string(),
        Value::Object(value::ObjectRef(MyId(1))),
    )
    .unwrap();

    tree.construct_object(MyId(2)).unwrap();
    tree.object_assign(
        MyId(1),
        "my key 2".to_string(),
        Value::Object(value::ObjectRef(MyId(2))),
    )
    .unwrap();

    // {"my key": {"my key 2": {}}}
    // ^          ^            ^
    // 0          1            2

    // let's attempt an assignment that causes a loop

    // first, unassign 1 from 0["my_key"]
    tree.object_assign(MyId(0), "my key".to_string(), Value::Int(123))
        .unwrap();

    // next, make the now-orphaned 1 a child of 2
    assert_eq!(
        Err(TreeError::EditWouldCauseCycle),
        tree.object_assign(
            MyId(2),
            "my key 3".to_string(),
            Value::Object(value::ObjectRef(MyId(1)))
        )
    );
}
