use crate::form::{
    array::ScalarArrayState,
    composite::{CompositeListState, CompositeState},
    key_value::KeyValueState,
};

#[derive(Debug, Clone)]
pub enum FieldValue {
    Text(String),
    Bool(bool),
    Enum {
        options: Vec<String>,
        selected: usize,
    },
    MultiSelect {
        options: Vec<String>,
        selected: Vec<bool>,
    },
    Array(String),
    ScalarArray(ScalarArrayState),
    Composite(CompositeState),
    CompositeList(CompositeListState),
    KeyValue(KeyValueState),
}

#[derive(Debug, Clone)]
pub struct CompositePopupData {
    pub options: Vec<String>,
    pub selected: usize,
    pub multi: bool,
    pub active: Vec<bool>,
}
