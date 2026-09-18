use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "web-types")]
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[cfg_attr(feature = "web-types", ts(export, export_to = "web/types/ui-ast.ts"))]
pub struct UiAst {
    pub roots: Vec<UiNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
pub struct UiNode {
    pub pointer: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown> | null"))]
    pub default_value: Option<Value>,
    pub visible_when: Option<VisibleWhen>,
    /// The control the author asked for, if they asked for one.
    ///
    /// `None` is the common case and means "let the frontend choose", which is
    /// what every frontend did before this hint existed. A frontend that does
    /// not recognise a control must fall back to its default for the node's
    /// shape rather than refuse to render the field.
    pub control: Option<FieldControl>,
    /// Numeric range and scale hints, for controls that have a range.
    ///
    /// Kept on the node rather than inside the kind so the same hints apply
    /// whether the value is a single number (a slider) or a pair of numbers (a
    /// range) — the two controls differ, the bounds do not.
    pub bounds: Option<FieldBounds>,
    pub kind: UiNodeKind,
}

/// Which control a frontend should render, authored with `x-control`.
///
/// The names describe the *interaction*, not a widget: a frontend is free to
/// render `select` as a native dropdown or a listbox, and `segmented` as
/// buttons or chips. What the schema pins down is the choice between one input
/// and another, which is the part an author actually has an opinion about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum FieldControl {
    /// Single-line text input. The default for a string.
    Text,
    /// Multi-line text area. Equivalent to `x-multiline: true`.
    Textarea,
    /// Dropdown. The default for an enum.
    Select,
    /// All options visible at once, as a row of buttons. Suits short enums.
    Segmented,
    /// Stacked radio buttons.
    Radio,
    /// Toggle. The default for a boolean.
    Switch,
    /// Labelled checkbox.
    Checkbox,
    /// Single-thumb slider. Needs numeric bounds to be meaningful.
    Slider,
    /// Dual-thumb slider for a pair of numbers, e.g. a min/max window.
    Range,
    /// Colour picker for a string holding a CSS colour.
    Color,
}

impl FieldControl {
    /// The keyword as it is spelled in a schema, for error messages.
    pub fn keyword(self) -> &'static str {
        match self {
            FieldControl::Text => "text",
            FieldControl::Textarea => "textarea",
            FieldControl::Select => "select",
            FieldControl::Segmented => "segmented",
            FieldControl::Radio => "radio",
            FieldControl::Switch => "switch",
            FieldControl::Checkbox => "checkbox",
            FieldControl::Slider => "slider",
            FieldControl::Range => "range",
            FieldControl::Color => "color",
        }
    }

    /// The schema shape this control can render, for error messages.
    pub fn expects(self) -> &'static str {
        match self {
            FieldControl::Text | FieldControl::Textarea | FieldControl::Color => "a string",
            FieldControl::Switch | FieldControl::Checkbox => "a boolean",
            FieldControl::Slider => "a number",
            FieldControl::Range => "an array of two numbers",
            FieldControl::Select | FieldControl::Segmented | FieldControl::Radio => "an enum",
        }
    }
}

/// Bounds and scale for a ranged control.
///
/// Absent bounds are not an error: a slider without a declared maximum simply
/// has nothing to slide along, and the frontend falls back to a plain input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
pub struct FieldBounds {
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    /// Increment between stops, from `multipleOf`.
    pub step: Option<f64>,
    /// Labelled stops along the track, from `x-slider-marks`.
    #[serde(default)]
    pub marks: Vec<SliderMark>,
}

impl FieldBounds {
    /// Whether there is enough here to draw a track.
    ///
    /// A slider needs both ends: with one bound the remaining end would have to
    /// be invented, and an invented maximum is worse than no slider at all.
    pub fn is_ranged(&self) -> bool {
        self.minimum.is_some() && self.maximum.is_some()
    }
}

/// A labelled stop on a slider track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
pub struct SliderMark {
    pub value: f64,
    pub label: Option<String>,
}

/// Conditional visibility for a node, tested against a sibling property of the
/// object that owns it.
///
/// Authored with the `x-visible-when` extension:
/// `{"x-visible-when": {"field": "mode", "op": "equals", "value": "custom"}}`
/// keeps the node hidden until the sibling `mode` holds the value `custom`,
/// while `"op": "contains"` does the same for a sibling array that has to
/// include the value. This is what lets the escape-hatch pattern — an enum with
/// an "other" option plus a companion free-text field — stop rendering a
/// permanently empty input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
pub struct VisibleWhen {
    pub field: String,
    pub op: VisibleWhenOp,
    #[cfg_attr(feature = "web-types", ts(type = "unknown"))]
    pub value: Value,
}

/// How a [`VisibleWhen`] rule compares the sibling property with its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum VisibleWhenOp {
    /// The sibling value is exactly `value`.
    Equals,
    /// The sibling value is an array holding `value` as one of its members.
    Contains,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiNodeKind {
    Field {
        scalar: ScalarKind,
        enum_options: Option<Vec<String>>,
        #[cfg_attr(feature = "web-types", ts(type = "unknown[] | null"))]
        enum_values: Option<Vec<Value>>,
        nullable: bool,
        multiline: bool,
    },
    Array {
        item: Box<UiNodeKind>,
        min_items: Option<u64>,
        max_items: Option<u64>,
    },
    KeyValue {
        template: Box<UiKeyValueNode>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
pub struct UiKeyValueNode {
    pub key_title: String,
    pub key_description: Option<String>,
    #[cfg_attr(feature = "web-types", ts(type = "unknown | null"))]
    pub key_default: Option<Value>,
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown>"))]
    pub key_schema: Value,
    pub value_title: String,
    pub value_description: Option<String>,
    #[cfg_attr(feature = "web-types", ts(type = "unknown | null"))]
    pub value_default: Option<Value>,
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown>"))]
    pub value_schema: Value,
    pub value_kind: Box<UiNodeKind>,
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown>"))]
    pub entry_schema: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum ScalarKind {
    String,
    Integer,
    Number,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[serde(rename_all = "snake_case")]
pub enum CompositeMode {
    OneOf,
    AnyOf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
pub struct UiVariant {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_object: bool,
    pub node: UiNodeKind,
    #[cfg_attr(feature = "web-types", ts(type = "Record<string, unknown>"))]
    pub schema: Value,
}
