use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::domain::FieldSchema;

use super::ComponentKind;
use crate::form::field::convert::adjust_numeric_value;

pub(crate) fn handle_text_edit(buffer: &mut String, schema: &FieldSchema, key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Left => adjust_numeric_value(buffer, &schema.kind, -1),
        KeyCode::Right => adjust_numeric_value(buffer, &schema.kind, 1),
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return false;
            }
            buffer.push(ch);
            true
        }
        KeyCode::Backspace => {
            buffer.pop();
            true
        }
        KeyCode::Delete => {
            buffer.clear();
            true
        }
        _ => false,
    }
}

pub(crate) fn format_collection_value(
    label: &str,
    len: usize,
    selection: Option<String>,
    hint: &str,
) -> String {
    if len == 0 {
        format!("{label}: empty {hint}")
    } else {
        let selected = selection.unwrap_or_else(|| "<no selection>".to_string());
        format!("{label}[{len}] • {selected} {hint}")
    }
}

pub(crate) fn list_hint_for(kind: ComponentKind) -> &'static str {
    match kind {
        ComponentKind::CompositeList | ComponentKind::KeyValue => {
            "(Ctrl+Left/Right select, Ctrl+E edit)"
        }
        ComponentKind::ScalarArray => "(Ctrl+Left/Right select, Ctrl+E edit)",
        _ => "",
    }
}
