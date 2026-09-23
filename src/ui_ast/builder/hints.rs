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
use crate::rich::MAX_SOURCE_BYTES;
use crate::ui_ast::types::{
    EnumDetail, FieldBounds, FieldControl, RichContent, SliderMark, UiNode, UiNodeKind,
};

/// `x-multiline: true` — render a string field as a multi-line text area.
pub(super) const MULTILINE: &str = "x-multiline";

/// `x-control: <name>` — the control the author wants for this node.
pub(super) const CONTROL: &str = "x-control";

/// `x-slider-marks: [...]` — labelled stops along a slider track.
pub(super) const SLIDER_MARKS: &str = "x-slider-marks";

/// `x-visible-when: {"field": <sibling>, "op": "equals" | "contains", "value": <value>}`
/// — keep a node hidden until a sibling property matches.
pub(super) const VISIBLE_WHEN: &str = "x-visible-when";

/// `x-options: [{label?, description?, content?}]` — per-option detail,
/// aligned with the enum values by index.
pub(super) const OPTIONS: &str = "x-options";

/// `x-content: {"type": "mermaid" | "svg" | "markdown", "source": <text>}` —
/// a figure or prose block attached to the node itself.
pub(super) const CONTENT: &str = "x-content";

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
        "mermaid" => FieldControl::Mermaid,
        other => bail!(
            "`{CONTROL}` names an unknown control `{other}`; expected one of \
             text, textarea, select, segmented, radio, switch, checkbox, slider, range, color, mermaid"
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

/// Per-option metadata from `x-options`, aligned with the enum values.
///
/// The entries must line up with the enum one-for-one: an `x-options` list
/// that drifts from its enum is a schema bug whose symptom would otherwise be
/// the wrong figure on the wrong option, which nothing downstream can catch.
pub(super) fn enum_details(
    schema: &SchemaObject,
    enum_values: Option<&Vec<Value>>,
) -> Result<Option<Vec<EnumDetail>>> {
    let Some(raw) = schema.extensions.get(OPTIONS) else {
        return Ok(None);
    };
    let Some(entries) = raw.as_array() else {
        bail!("`{OPTIONS}` must be an array, found `{raw}`");
    };
    let Some(values) = enum_values else {
        bail!("`{OPTIONS}` needs an enum to describe, but the property declares none");
    };
    if entries.len() != values.len() {
        bail!(
            "`{OPTIONS}` has {} entries but the enum has {} values",
            entries.len(),
            values.len()
        );
    }

    let details = entries
        .iter()
        .map(|entry| {
            let Some(fields) = entry.as_object() else {
                bail!("`{OPTIONS}` entries must be objects, found `{entry}`");
            };
            Ok(EnumDetail {
                label: optional_string(fields.get("label"), "label", OPTIONS)?,
                description: optional_string(fields.get("description"), "description", OPTIONS)?,
                content: rich_content(fields.get("content"), OPTIONS)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // An all-empty list is the author's way of writing nothing; keep it out
    // of the AST so the wire stays free of placeholder detail rows.
    let meaningful = details.iter().any(|detail| {
        detail.label.is_some() || detail.description.is_some() || detail.content.is_some()
    });
    Ok(meaningful.then_some(details))
}

/// The figure or prose block declared with `x-content`, if any.
pub(super) fn node_content(schema: &SchemaObject) -> Result<Option<RichContent>> {
    rich_content(schema.extensions.get(CONTENT), CONTENT)
}

fn optional_string(raw: Option<&Value>, field: &str, keyword: &str) -> Result<Option<String>> {
    match raw {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(other) => bail!("`{keyword}` entry {field} must be a string, found `{other}`"),
    }
}

fn rich_content(raw: Option<&Value>, keyword: &str) -> Result<Option<RichContent>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let content: RichContent = serde_json::from_value(raw.clone())
        .map_err(|error| anyhow::anyhow!("`{keyword}` is malformed ({error}), found `{raw}`"))?;
    if content.source().is_empty() {
        bail!("`{keyword}` needs a non-empty `source`");
    }
    if content.source().len() > MAX_SOURCE_BYTES {
        bail!(
            "`{keyword}` source is {} bytes; the limit is {MAX_SOURCE_BYTES}",
            content.source().len()
        );
    }
    Ok(Some(content))
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
        content: node_content(schema)?,
        kind,
    };
    check_control_fits(&node)?;
    check_details_fit(&node)?;
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
        (
            UiNodeKind::Field { scalar, .. },
            FieldControl::Text
            | FieldControl::Textarea
            | FieldControl::Color
            | FieldControl::Mermaid,
        ) => matches!(scalar, super::ScalarKind::String),
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

/// Rejects `x-options` figures the declared control cannot show.
///
/// A segmented row has no room for anything but its label; declaring a
/// diagram on one would silently lose it, which is the same class of bug as a
/// typo'd control name. The check fires only when there is something to lose,
/// so plain labelled options stay valid on every enum control.
fn check_details_fit(node: &UiNode) -> Result<()> {
    if node.control != Some(FieldControl::Segmented) {
        return Ok(());
    }
    if let UiNodeKind::Field {
        enum_details: Some(details),
        ..
    } = &node.kind
        && details.iter().any(|detail| detail.content.is_some())
    {
        bail!(
            "`{OPTIONS}` content needs room to render; `{CONTROL}: \"segmented\"` has none. \
             Use `radio` or `select` so each option can show its figure"
        );
    }
    Ok(())
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
