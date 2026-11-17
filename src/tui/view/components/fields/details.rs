use super::text_layout::{wrap_with_prefix, wrap_with_prefix_limited};
use crate::tui::{
    model::{CompositeMode, FieldKind, FieldSchema},
    state::FieldState,
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub(crate) fn meta_lines(
    field: &FieldState,
    is_selected: bool,
    max_width: u16,
) -> Vec<Line<'static>> {
    let mut parts = Vec::new();
    parts.push(format!("type: {}", field_type_label(&field.schema.kind)));

    if let Some(control) = field.schema.presentation.control {
        parts.push(format!("control: {}", control.keyword()));
    }
    let constraints = extract_constraints(&field.schema);
    if !constraints.is_empty() {
        parts.push(format!("constraints: {}", constraints.join(", ")));
    }

    if let Some(desc) = field
        .schema
        .description
        .as_ref()
        .map(|d| d.trim())
        .filter(|d| !d.is_empty())
    {
        parts.push(format!("desc: {}", desc));
    }
    let content = parts.join(" | ");
    if content.is_empty() {
        return Vec::new();
    }

    let style = if is_selected {
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    wrap_with_prefix(&content, "  ", max_width, style, style)
}

pub(crate) fn error_lines(field: &FieldState, max_width: u16) -> Option<Vec<Line<'static>>> {
    field.error.as_ref().map(|message| {
        let mut lines = Vec::new();
        lines.push(Line::from(Span::styled(
            "  Error:",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        let error_style = Style::default().fg(Color::Red);
        lines.extend(wrap_with_prefix_limited(
            message,
            "    ",
            max_width,
            error_style,
            error_style,
            3,
        ));
        lines
    })
}

pub(super) fn field_type_label(kind: &FieldKind) -> String {
    match kind {
        FieldKind::String => "string".to_string(),
        FieldKind::Integer => "integer".to_string(),
        FieldKind::Number => "number".to_string(),
        FieldKind::Boolean => "boolean".to_string(),
        FieldKind::Enum { .. } => "enum".to_string(),
        FieldKind::Nullable(inner) => format!("{} | null", field_type_label(inner)),
        FieldKind::Array(inner) => match inner.as_ref() {
            FieldKind::Json => "object[]".to_string(),
            FieldKind::Composite(comp) => {
                // Get more descriptive label for composite arrays
                match &comp.mode {
                    CompositeMode::OneOf => {
                        if comp.variants.len() == 1 {
                            format!("{}[]", comp.variants[0].title)
                        } else {
                            "choice[]".to_string()
                        }
                    }
                    CompositeMode::AnyOf => "multi-choice[]".to_string(),
                }
            }
            _ => format!("{}[]", field_type_label(inner)),
        },
        FieldKind::Json => "object".to_string(),
        FieldKind::Composite(comp) => match &comp.mode {
            CompositeMode::OneOf => "choice".to_string(),
            CompositeMode::AnyOf => "multi-choice".to_string(),
        },
        FieldKind::KeyValue(_) => "map".to_string(),
    }
}

fn extract_constraints(schema: &FieldSchema) -> Vec<String> {
    let mut constraints = Vec::new();
    if let Some(bounds) = &schema.presentation.bounds {
        if let Some(min) = bounds.minimum {
            constraints.push(format!("min: {min}"));
        }
        if let Some(max) = bounds.maximum {
            constraints.push(format!("max: {max}"));
        }
        if let Some(step) = bounds.step {
            constraints.push(format!("multipleOf: {step}"));
        }
    }

    // format 约束
    if let Some(format) = schema.metadata.get("format")
        && let Some(format_str) = format.as_str()
    {
        constraints.push(format!("format: {}", format_str));
    }

    // pattern 约束
    if let Some(pattern) = schema.metadata.get("pattern")
        && let Some(pattern_str) = pattern.as_str()
    {
        let truncated = if pattern_str.chars().count() > 30 {
            format!("{}...", pattern_str.chars().take(27).collect::<String>())
        } else {
            pattern_str.to_string()
        };
        constraints.push(format!("pattern: {}", truncated));
    }

    // 字符串长度约束
    if matches!(schema.kind, FieldKind::String | FieldKind::Nullable(_)) {
        if let Some(min) = schema.metadata.get("minLength").and_then(|v| v.as_u64()) {
            if let Some(max) = schema.metadata.get("maxLength").and_then(|v| v.as_u64()) {
                constraints.push(format!("length: {}..{}", min, max));
            } else {
                constraints.push(format!("minLength: {}", min));
            }
        } else if let Some(max) = schema.metadata.get("maxLength").and_then(|v| v.as_u64()) {
            constraints.push(format!("maxLength: {}", max));
        }
    }

    // 数值范围约束
    if matches!(
        schema.kind,
        FieldKind::Integer | FieldKind::Number | FieldKind::Nullable(_)
    ) {
        let min = schema
            .metadata
            .get("minimum")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)));
        let max = schema
            .metadata
            .get("maximum")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)));

        if let (Some(min_val), Some(max_val)) = (min, max) {
            if matches!(schema.kind, FieldKind::Integer) {
                constraints.push(format!("range: {}..{}", min_val as i64, max_val as i64));
            } else {
                constraints.push(format!("range: {:.2}..{:.2}", min_val, max_val));
            }
        } else if let Some(min_val) = min {
            constraints.push(format!(
                "min: {}",
                if matches!(schema.kind, FieldKind::Integer) {
                    format!("{}", min_val as i64)
                } else {
                    format!("{:.2}", min_val)
                }
            ));
        } else if let Some(max_val) = max {
            constraints.push(format!(
                "max: {}",
                if matches!(schema.kind, FieldKind::Integer) {
                    format!("{}", max_val as i64)
                } else {
                    format!("{:.2}", max_val)
                }
            ));
        }

        // multipleOf 约束
        if let Some(multiple) = schema.metadata.get("multipleOf").and_then(|v| v.as_f64()) {
            constraints.push(format!("multipleOf: {}", multiple));
        }
    }

    // 数组项数约束
    if matches!(schema.kind, FieldKind::Array(_)) {
        if let Some(min) = schema.metadata.get("minItems").and_then(|v| v.as_u64()) {
            if let Some(max) = schema.metadata.get("maxItems").and_then(|v| v.as_u64()) {
                constraints.push(format!("items: {}..{}", min, max));
            } else {
                constraints.push(format!("minItems: {}", min));
            }
        } else if let Some(max) = schema.metadata.get("maxItems").and_then(|v| v.as_u64()) {
            constraints.push(format!("maxItems: {}", max));
        }

        if let Some(unique) = schema.metadata.get("uniqueItems").and_then(|v| v.as_bool())
            && unique
        {
            constraints.push("unique".to_string());
        }
    }

    constraints
}
