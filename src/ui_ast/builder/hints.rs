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

/// `x-multiline: true` — render a string field as a multi-line text area.
pub(super) const MULTILINE: &str = "x-multiline";

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
