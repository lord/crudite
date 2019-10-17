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
