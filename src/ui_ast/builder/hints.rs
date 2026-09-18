//! Presentation hints carried by `x-` schema extensions.
//!
//! These keywords never affect validation — they only tell a frontend how to
//! render a node. Keeping them here means the set of supported hints is
//! discoverable in one place, and malformed hints fail loudly instead of
//! silently degrading into a UI that looks nothing like the schema asked for.

use anyhow::{Result, bail};
use indexmap::IndexMap;
use serde_json::Value;

use super::{Schema, SchemaObject, VisibleWhen};
use crate::ui_ast::types::{FieldBounds, FieldControl, SliderMark, UiNode, UiNodeKind};

/// `x-multiline: true` — render a string field as a multi-line text area.
pub(super) const MULTILINE: &str = "x-multiline";

/// `x-control: <name>` — the control the author wants for this node.
pub(super) const CONTROL: &str = "x-control";

/// `x-slider-marks: [...]` — labelled stops along a slider track.
pub(super) const SLIDER_MARKS: &str = "x-slider-marks";

/// `x-visible-when: {"field": <sibling>, "op": "equals" | "contains", "value": <value>}`
/// — keep a node hidden until a sibling property matches.
pub(super) const VISIBLE_WHEN: &str = "x-visible-when";

/// Whether the field asks for a multi-line text area.
pub(super) fn is_multiline(schema: &SchemaObject) -> Result<bool> {
    match schema.extensions.get(MULTILINE) {
        None => Ok(false),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(other) => bail!("`{MULTILINE}` must be a boolean, found `{other}`"),
    }
}

/// The control named by `x-control`, if the node names one.
///
/// An unknown name is an error rather than a silent fallback: a typo like
/// `x-control: "silder"` would otherwise render a plain input and look like the
/// hint had simply been ignored, which is the hardest kind of bug to spot in a
/// schema you are reading.
pub(super) fn field_control(schema: &SchemaObject) -> Result<Option<FieldControl>> {
    let Some(raw) = schema.extensions.get(CONTROL) else {
        return Ok(None);
    };
    let Some(name) = raw.as_str() else {
        bail!("`{CONTROL}` must be a string, found `{raw}`");
    };
    let control = match name {
        "text" => FieldControl::Text,
        "textarea" => FieldControl::Textarea,
        "select" => FieldControl::Select,
        "segmented" => FieldControl::Segmented,
        "radio" => FieldControl::Radio,
        "switch" => FieldControl::Switch,
        "checkbox" => FieldControl::Checkbox,
        "slider" => FieldControl::Slider,
        "range" => FieldControl::Range,
        "color" => FieldControl::Color,
        other => bail!(
            "`{CONTROL}` names an unknown control `{other}`; expected one of \
             text, textarea, select, segmented, radio, switch, checkbox, slider, range, color"
        ),
    };
    Ok(Some(control))
}

/// Numeric range and scale for this node, assembled from the standard keywords
/// plus `x-slider-marks`.
///
/// `minimum`/`maximum`/`multipleOf` are ordinary validation keywords that the
/// schema model keeps in `extensions` rather than modelling, so they are read
/// from there. Returning `None` when nothing is declared keeps the common case
/// free of an empty struct in every serialised node.
pub(super) fn field_bounds(schema: &SchemaObject) -> Result<Option<FieldBounds>> {
    let number = |keyword: &str| -> Result<Option<f64>> {
        match schema.extensions.get(keyword) {
            None => Ok(None),
            Some(Value::Number(value)) => Ok(value.as_f64()),
            Some(other) => bail!("`{keyword}` must be a number, found `{other}`"),
        }
    };

    let bounds = FieldBounds {
        minimum: number("minimum")?,
        maximum: number("maximum")?,
        step: number("multipleOf")?,
        marks: slider_marks(schema)?,
    };

    let declared = bounds.minimum.is_some()
        || bounds.maximum.is_some()
        || bounds.step.is_some()
        || !bounds.marks.is_empty();
    Ok(declared.then_some(bounds))
}

/// Labelled stops along a slider track.
///
/// Accepts either a bare number or `{"value": <n>, "label": <text>}`, so the
/// common case of evenly spaced stops stays readable:
/// `"x-slider-marks": [0, 50, 100]`.
fn slider_marks(schema: &SchemaObject) -> Result<Vec<SliderMark>> {
    let Some(raw) = schema.extensions.get(SLIDER_MARKS) else {
        return Ok(Vec::new());
    };
    let Some(entries) = raw.as_array() else {
        bail!("`{SLIDER_MARKS}` must be an array, found `{raw}`");
    };

    entries
        .iter()
        .map(|entry| match entry {
            Value::Number(value) => {
                let value = value
                    .as_f64()
                    .ok_or_else(|| anyhow::anyhow!("`{SLIDER_MARKS}` has a value out of range"))?;
                Ok(SliderMark { value, label: None })
            }
            Value::Object(fields) => {
                let value = fields.get("value").and_then(Value::as_f64).ok_or_else(|| {
                    anyhow::anyhow!("`{SLIDER_MARKS}` entry `{entry}` needs a numeric `value`")
                })?;
                let label = match fields.get("label") {
                    None | Some(Value::Null) => None,
                    Some(Value::String(text)) => Some(text.clone()),
                    Some(other) => {
                        bail!("`{SLIDER_MARKS}` entry label must be a string, found `{other}`")
                    }
                };
                Ok(SliderMark { value, label })
            }
            other => bail!("`{SLIDER_MARKS}` entries must be numbers or objects, found `{other}`"),
        })
        .collect()
}

/// Assemble a node, attaching every `x-` hint it declares.
///
/// Every construction path goes through here so that a hint cannot be silently
/// dropped by one branch while working in another — which is exactly how
/// `x-multiline` used to behave across the three places a field node is built.
pub(super) fn node(
    schema: &SchemaObject,
    pointer: String,
    required: bool,
    default_value: Option<Value>,
    kind: UiNodeKind,
) -> Result<UiNode> {
    let node = UiNode {
        pointer,
        title: super::schema_helpers::schema_title(schema),
        description: super::schema_helpers::schema_description(schema),
        required,
        default_value,
        // Filled in by the caller once the containing object is known: a rule
        // can only be checked against the siblings it points at.
        visible_when: None,
        control: field_control(schema)?,
        bounds: field_bounds(schema)?,
        kind,
    };
    check_control_fits(&node)?;
    Ok(node)
}

/// Rejects a control that cannot render the value it was attached to.
///
/// A slider on a string or a colour picker on a boolean are schema bugs, and
/// the frontend cannot paper over them — it would either render nothing or
/// invent a coercion. Better to stop at load time, where the pointer still
/// points at the offending property.
fn check_control_fits(node: &UiNode) -> Result<()> {
    let Some(control) = node.control else {
        return Ok(());
    };
    let pointer = if node.pointer.is_empty() {
        "(root)"
    } else {
        &node.pointer
    };

    let fits = match (&node.kind, control) {
        (UiNodeKind::Field { scalar, .. }, FieldControl::Text | FieldControl::Textarea)
        | (UiNodeKind::Field { scalar, .. }, FieldControl::Color) => {
            matches!(scalar, super::ScalarKind::String)
        }
        (UiNodeKind::Field { scalar, .. }, FieldControl::Switch | FieldControl::Checkbox) => {
            matches!(scalar, super::ScalarKind::Boolean)
        }
        (UiNodeKind::Field { scalar, .. }, FieldControl::Slider) => matches!(
            scalar,
            super::ScalarKind::Integer | super::ScalarKind::Number
        ),
        (
            UiNodeKind::Field { enum_values, .. },
            FieldControl::Select | FieldControl::Segmented | FieldControl::Radio,
        ) => enum_values
            .as_ref()
            .is_some_and(|values| !values.is_empty()),
        (
            UiNodeKind::Array {
                item,
                min_items,
                max_items,
            },
            FieldControl::Range,
        ) => {
            let numeric = matches!(
                item.as_ref(),
                UiNodeKind::Field {
                    scalar: super::ScalarKind::Integer | super::ScalarKind::Number,
                    ..
                }
            );
            numeric && *min_items == Some(2) && *max_items == Some(2)
        }
        _ => false,
    };

    if fits {
        return Ok(());
    }

    let expects = match control {
        FieldControl::Range => {
            "an array of two numbers (declare `minItems: 2` and `maxItems: 2`)".to_string()
        }
        other => other.expects().to_string(),
    };
    bail!(
        "`{CONTROL}: \"{}\"` on `{pointer}` needs {expects}, but the property is {}",
        control.keyword(),
        describe_kind(&node.kind)
    )
}

/// A short description of what a node actually is, for error messages.
fn describe_kind(kind: &UiNodeKind) -> String {
    match kind {
        UiNodeKind::Field { scalar, .. } => match scalar {
            super::ScalarKind::String => "a string".to_string(),
            super::ScalarKind::Integer => "an integer".to_string(),
            super::ScalarKind::Number => "a number".to_string(),
            super::ScalarKind::Boolean => "a boolean".to_string(),
        },
        UiNodeKind::Array { .. } => "an array".to_string(),
        UiNodeKind::Object { .. } => "an object".to_string(),
        UiNodeKind::KeyValue { .. } => "a key/value map".to_string(),
        UiNodeKind::Composite { .. } => "a oneOf/anyOf".to_string(),
    }
}

/// The visibility rule declared on this schema, if any.
pub(super) fn visible_when(schema: &SchemaObject) -> Result<Option<VisibleWhen>> {
    let Some(raw) = schema.extensions.get(VISIBLE_WHEN) else {
        return Ok(None);
    };
    // Quote the raw value back: it is what identifies the offending node, since
    // the parser runs before the node has a pointer to report.
    serde_json::from_value(raw.clone())
        .map(Some)
        .map_err(|error| anyhow::anyhow!("`{VISIBLE_WHEN}` is malformed ({error}), found `{raw}`"))
}

/// Rejects a rule that points at a property the containing object does not
/// declare. Such a typo would hide the node forever, with nothing in the UI to
/// explain why it never showed up.
pub(super) fn check_sibling(
    rule: &VisibleWhen,
    properties: &IndexMap<String, Schema>,
    pointer: &str,
) -> Result<()> {
    if properties.contains_key(&rule.field) {
        return Ok(());
    }
    bail!(
        "`{VISIBLE_WHEN}` on `{pointer}` points at `{}`, which is not a property of the containing object",
        rule.field
    )
}
