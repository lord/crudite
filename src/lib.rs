pub mod tree;

enum Node {
    List(),
}

enum Op {
    Assign,
    Remove,
    InsertAfter,
    MakeList,
    MakeMap,
    MakeVal,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
