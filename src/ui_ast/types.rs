use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "web")]
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "web", derive(TS))]
#[cfg_attr(feature = "web", ts(export, export_to = "web/types/ui-ast.ts"))]
pub struct UiAst {
    pub roots: Vec<UiNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "web", derive(TS))]
pub struct UiNode {
    pub pointer: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    #[cfg_attr(feature = "web", ts(type = "Record<string, unknown> | null"))]
    pub default_value: Option<Value>,
    pub kind: UiNodeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "web", derive(TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiNodeKind {
    Field {
        scalar: ScalarKind,
        enum_options: Option<Vec<String>>,
        #[cfg_attr(feature = "web", ts(type = "unknown[] | null"))]
        enum_values: Option<Vec<Value>>,
    },
    Array {
        item: Box<UiNodeKind>,
        min_items: Option<u64>,
        max_items: Option<u64>,
    },
    Composite {
        mode: CompositeMode,
        allow_multiple: bool,
        variants: Vec<UiVariant>,
    },
    Object {
        children: Vec<UiNode>,
        required: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "web", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum ScalarKind {
    String,
    Integer,
    Number,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "web", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum CompositeMode {
    OneOf,
    AnyOf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "web", derive(TS))]
pub struct UiVariant {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_object: bool,
    pub node: UiNodeKind,
    #[cfg_attr(feature = "web", ts(type = "Record<string, unknown>"))]
    pub schema: Value,
}
