use serde_json::{Map, Value};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const MAX_STRING_VISIBLE: usize = 36;
const MAX_INLINE_STRING_VISIBLE: usize = 24;
const MAX_COLLECTION_ITEMS: usize = 3;
const MAX_NESTED_DEPTH: usize = 2;

pub(crate) fn summarize_value(value: &Value) -> String {
    summarize_value_impl(value, 0, false)
}

pub(crate) fn summarize_inline_value(value: &Value) -> String {
    summarize_value_impl(value, 0, true)
}

fn summarize_value_impl(value: &Value, depth: usize, inline_string: bool) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(num) => num.to_string(),
        Value::String(text) => summarize_string(text, inline_string),
        Value::Array(items) => summarize_array(items, depth),
        Value::Object(map) => summarize_object(map, depth),
    }
}

fn summarize_array(items: &[Value], depth: usize) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    if depth >= MAX_NESTED_DEPTH {
        return format!("[{} items]", items.len());
    }

    let mut parts = items
        .iter()
        .take(MAX_COLLECTION_ITEMS)
        .map(|value| summarize_value_impl(value, depth + 1, true))
        .collect::<Vec<_>>();
    if items.len() > MAX_COLLECTION_ITEMS {
        parts.push("…".to_string());
    }
    truncate_visible(&format!("[{}]", parts.join(", ")), MAX_STRING_VISIBLE)
}

fn summarize_object(map: &Map<String, Value>, depth: usize) -> String {
    if map.is_empty() {
        return "{}".to_string();
    }
    if depth >= MAX_NESTED_DEPTH {
        return format!("{{{} keys}}", map.len());
    }

    let mut parts = map
        .iter()
        .take(MAX_COLLECTION_ITEMS)
        .map(|(key, value)| format!("{key}: {}", summarize_value_impl(value, depth + 1, true)))
        .collect::<Vec<_>>();
    if map.len() > MAX_COLLECTION_ITEMS {
        parts.push("…".to_string());
    }
    truncate_visible(&format!("{{ {} }}", parts.join(", ")), MAX_STRING_VISIBLE)
}

fn summarize_string(text: &str, inline: bool) -> String {
    if inline {
        truncate_visible(text, MAX_INLINE_STRING_VISIBLE)
    } else {
        format!("\"{}\"", truncate_display_string(text))
    }
}

fn truncate_display_string(text: &str) -> String {
    const TRUNCATE_TO: usize = MAX_STRING_VISIBLE - 12;
    if UnicodeWidthStr::width(text) > MAX_STRING_VISIBLE {
        let mut truncated = String::new();
        for ch in text.chars().take(TRUNCATE_TO) {
            truncated.push(ch);
        }
        truncated.push('…');
        truncated
    } else {
        text.to_string()
    }
}

fn truncate_visible(text: &str, max_visible: usize) -> String {
    if UnicodeWidthStr::width(text) <= max_visible {
        return text.to_string();
    }

    let mut out = String::new();
    let mut width = 0usize;
    let limit = max_visible.saturating_sub(1);
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > limit {
            break;
        }
        out.push(ch);
        width += ch_width;
    }
    out.push('…');
    out
}
