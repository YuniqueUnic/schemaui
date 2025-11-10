use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;

use crate::domain::{FieldKind, FieldSchema};

use super::{
    composite::{
        CompositeEditorSession, CompositeListEditorContext, CompositeListState, CompositeState,
    },
    error::FieldCoercionError,
    key_value::{KeyValueEditorContext, KeyValueEditorSession, KeyValueState},
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

#[derive(Debug, Clone)]
pub struct FieldState {
    pub schema: FieldSchema,
    pub value: FieldValue,
    pub dirty: bool,
    pub error: Option<String>,
}

impl FieldState {
    pub fn from_schema(schema: FieldSchema) -> Self {
        let value =
            match &schema.kind {
                FieldKind::String | FieldKind::Integer | FieldKind::Number | FieldKind::Json => {
                    FieldValue::Text(default_text(&schema))
                }
                FieldKind::Boolean => {
                    let default = schema
                        .default
                        .as_ref()
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    FieldValue::Bool(default)
                }
                FieldKind::Enum(options) => {
                    let default_value = schema
                        .default
                        .as_ref()
                        .map(value_to_string)
                        .and_then(|value| if value.is_empty() { None } else { Some(value) })
                        .unwrap_or_else(|| options.first().cloned().unwrap_or_default());
                    let selected = options
                        .iter()
                        .position(|item| item == &default_value)
                        .unwrap_or(0);
                    FieldValue::Enum {
                        options: options.clone(),
                        selected,
                    }
                }
                FieldKind::Array(inner) => match inner.as_ref() {
                    FieldKind::Enum(options) => {
                        let defaults = schema
                            .default
                            .as_ref()
                            .and_then(|value| value.as_array())
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_string)
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        let mut selected = vec![false; options.len()];
                        for (idx, option) in options.iter().enumerate() {
                            if defaults.iter().any(|value| value == option) {
                                selected[idx] = true;
                            }
                        }
                        FieldValue::MultiSelect {
                            options: options.clone(),
                            selected,
                        }
                    }
                    FieldKind::Composite(meta) => FieldValue::CompositeList(
                        CompositeListState::new(&schema.pointer, meta, schema.default.as_ref()),
                    ),
                    _ => {
                        let default = schema
                            .default
                            .as_ref()
                            .and_then(|value| value.as_array())
                            .map(|items| array_to_string(items))
                            .unwrap_or_default();
                        FieldValue::Array(default)
                    }
                },
                FieldKind::Composite(meta) => {
                    FieldValue::Composite(CompositeState::new(&schema.pointer, meta))
                }
                FieldKind::KeyValue(template) => FieldValue::KeyValue(KeyValueState::new(
                    &schema.pointer,
                    template,
                    schema.default.as_ref(),
                )),
            };

        let mut field = FieldState {
            schema,
            value,
            dirty: false,
            error: None,
        };

        if let Some(default) = field.schema.default.clone() {
            field.seed_value(&default);
        }

        field
    }

    pub fn seed_value(&mut self, value: &Value) {
        match (&mut self.value, value) {
            (FieldValue::Text(buffer), Value::String(text)) => {
                *buffer = text.clone();
            }
            (FieldValue::Text(buffer), Value::Number(num)) => {
                *buffer = num.to_string();
            }
            (FieldValue::Bool(current), Value::Bool(flag)) => {
                *current = *flag;
            }
            (FieldValue::Enum { options, selected }, Value::String(choice)) => {
                if let Some(idx) = options.iter().position(|opt| opt == choice) {
                    *selected = idx;
                }
            }
            (FieldValue::MultiSelect { options, selected }, Value::Array(items)) => {
                let mut flags = vec![false; options.len()];
                for item in items {
                    if let Some(label) = item.as_str() {
                        if let Some(pos) = options.iter().position(|opt| opt == label) {
                            flags[pos] = true;
                        }
                    }
                }
                if selected.len() == flags.len() {
                    selected.clone_from_slice(&flags);
                }
            }
            (FieldValue::Array(buffer), Value::Array(items)) => {
                *buffer = array_to_string(items);
            }
            (FieldValue::Composite(state), Value::Object(_)) => {
                let _ = state.seed_from_value(value);
            }
            (FieldValue::CompositeList(state), Value::Array(items)) => {
                state.seed_entries_from_array(items);
            }
            (FieldValue::KeyValue(state), Value::Object(map)) => {
                state.seed_entries_from_object(map);
            }
            _ => {}
        }

        self.dirty = false;
        self.error = None;
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> bool {
        match &mut self.value {
            FieldValue::Text(buffer) => match key.code {
                KeyCode::Left => {
                    if adjust_numeric_value(buffer, &self.schema.kind, -1) {
                        self.after_edit();
                        true
                    } else {
                        false
                    }
                }
                KeyCode::Right => {
                    if adjust_numeric_value(buffer, &self.schema.kind, 1) {
                        self.after_edit();
                        true
                    } else {
                        false
                    }
                }
                KeyCode::Char(c) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        return false;
                    }
                    buffer.push(c);
                    self.after_edit();
                    true
                }
                KeyCode::Backspace => {
                    buffer.pop();
                    self.after_edit();
                    true
                }
                KeyCode::Delete => {
                    buffer.clear();
                    self.after_edit();
                    true
                }
                _ => false,
            },
            FieldValue::Array(buffer) => match key.code {
                KeyCode::Char(c) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        return false;
                    }
                    buffer.push(c);
                    self.after_edit();
                    true
                }
                KeyCode::Backspace => {
                    buffer.pop();
                    self.after_edit();
                    true
                }
                KeyCode::Delete => {
                    buffer.clear();
                    self.after_edit();
                    true
                }
                _ => false,
            },
            FieldValue::Bool(value) => match key.code {
                KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right => {
                    *value = !*value;
                    self.after_edit();
                    true
                }
                _ => false,
            },
            FieldValue::Enum { options, selected } => match key.code {
                KeyCode::Up | KeyCode::Left => {
                    if *selected == 0 {
                        *selected = options.len().saturating_sub(1);
                    } else {
                        *selected -= 1;
                    }
                    self.after_edit();
                    true
                }
                KeyCode::Down | KeyCode::Right => {
                    if !options.is_empty() {
                        *selected = (*selected + 1) % options.len();
                    }
                    self.after_edit();
                    true
                }
                _ => false,
            },
            FieldValue::MultiSelect { .. } => false,
            FieldValue::Composite(_) => false,
            FieldValue::CompositeList(_) => false,
            FieldValue::KeyValue(_) => false,
        }
    }

    pub fn set_bool(&mut self, value: bool) {
        if let FieldValue::Bool(current) = &mut self.value {
            if *current != value {
                *current = value;
                self.after_edit();
            }
        }
    }

    pub fn set_enum_selected(&mut self, index: usize) {
        if let FieldValue::Enum { options, selected } = &mut self.value {
            if options.is_empty() {
                return;
            }
            let bounded = index.min(options.len() - 1);
            if *selected != bounded {
                *selected = bounded;
                self.after_edit();
            }
        }
    }

    pub fn set_multi_selection(&mut self, selections: &[bool]) {
        if let FieldValue::MultiSelect { selected, .. } = &mut self.value {
            if selected.len() == selections.len() && selected != selections {
                selected.clone_from_slice(selections);
                self.after_edit();
            }
        }
    }

    pub fn composite_popup(&self) -> Option<CompositePopupData> {
        if let FieldValue::Composite(state) = &self.value {
            let options = state.option_titles();
            if options.is_empty() {
                return None;
            }
            let selected = state.selected_index().unwrap_or(0).min(options.len() - 1);
            let active = state.active_flags();
            let multi = state.is_multi();
            return Some(CompositePopupData {
                options,
                selected,
                multi,
                active,
            });
        }
        None
    }

    pub fn active_composite_variants(&self) -> Vec<usize> {
        if let FieldValue::Composite(state) = &self.value {
            state.active_indices()
        } else {
            Vec::new()
        }
    }

    pub fn is_composite_list(&self) -> bool {
        matches!(
            self.value,
            FieldValue::CompositeList(_) | FieldValue::KeyValue(_)
        )
    }

    pub fn composite_list_select_entry(&mut self, delta: i32) -> bool {
        match &mut self.value {
            FieldValue::CompositeList(state) => state.select(delta),
            FieldValue::KeyValue(state) => state.select(delta),
            _ => false,
        }
    }

    pub fn composite_list_selected_label(&self) -> Option<String> {
        match &self.value {
            FieldValue::CompositeList(state) => state.selected_label(),
            FieldValue::KeyValue(state) => state.selected_label(),
            _ => None,
        }
    }

    pub fn composite_list_panel(&self) -> Option<(Vec<String>, usize)> {
        match &self.value {
            FieldValue::CompositeList(state) => {
                state.selected_index().map(|idx| (state.summaries(), idx))
            }
            FieldValue::KeyValue(state) => state.panel(),
            _ => None,
        }
    }

    pub fn composite_list_selected_index(&self) -> Option<usize> {
        match &self.value {
            FieldValue::CompositeList(state) => state.selected_index(),
            FieldValue::KeyValue(state) => state.selected_index(),
            _ => None,
        }
    }

    pub fn composite_list_add_entry(&mut self) -> bool {
        match &mut self.value {
            FieldValue::CompositeList(state) => {
                state.add_entry();
                self.after_edit();
                true
            }
            FieldValue::KeyValue(state) => {
                if state.add_entry() {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn composite_list_remove_entry(&mut self) -> bool {
        match &mut self.value {
            FieldValue::CompositeList(state) => {
                if state.remove_selected() {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            FieldValue::KeyValue(state) => {
                if state.remove_selected() {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn composite_list_move_entry(&mut self, delta: i32) -> bool {
        match &mut self.value {
            FieldValue::CompositeList(state) => {
                if state.move_selected(delta) {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            FieldValue::KeyValue(state) => {
                if state.move_selected(delta) {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn open_composite_editor(
        &mut self,
        variant_index: usize,
    ) -> Result<CompositeEditorSession, FieldCoercionError> {
        if let FieldValue::Composite(state) = &self.value {
            state.take_editor_session(&self.schema.pointer, variant_index)
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not a composite".to_string(),
            })
        }
    }

    pub fn open_composite_list_editor(
        &mut self,
    ) -> Result<CompositeListEditorContext, FieldCoercionError> {
        if let FieldValue::CompositeList(state) = &mut self.value {
            state.open_selected_editor()
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not a composite list".to_string(),
            })
        }
    }

    pub fn open_key_value_editor(&mut self) -> Result<KeyValueEditorContext, FieldCoercionError> {
        if let FieldValue::KeyValue(state) = &mut self.value {
            state.open_selected_editor()
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not a key/value map".to_string(),
            })
        }
    }

    pub fn close_composite_editor(&mut self, session: CompositeEditorSession, mark_dirty: bool) {
        if let FieldValue::Composite(state) = &self.value {
            state.restore_editor_session(session);
            if mark_dirty {
                self.after_edit();
            }
        }
    }

    pub fn close_composite_list_editor(
        &mut self,
        entry_index: usize,
        session: CompositeEditorSession,
        mark_dirty: bool,
    ) {
        if let FieldValue::CompositeList(state) = &mut self.value {
            state.restore_entry_editor(entry_index, session);
            if mark_dirty {
                self.after_edit();
            }
        }
    }

    pub fn close_key_value_editor(
        &mut self,
        entry_index: usize,
        session: &KeyValueEditorSession,
        mark_dirty: bool,
    ) -> Result<bool, FieldCoercionError> {
        if let FieldValue::KeyValue(state) = &mut self.value {
            let changed = state.apply_editor_session(entry_index, session)?;
            if mark_dirty && changed {
                self.after_edit();
            }
            Ok(changed)
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not a key/value map".to_string(),
            })
        }
    }

    pub fn apply_composite_selection(&mut self, selection: usize, multi_flags: Option<Vec<bool>>) {
        if let FieldValue::Composite(state) = &mut self.value {
            let changed = if state.is_multi() {
                if let Some(flags) = multi_flags {
                    state.apply_multi(&flags)
                } else {
                    false
                }
            } else {
                state.apply_single(selection)
            };
            if changed {
                self.after_edit();
            }
        }
    }

    pub fn multi_states(&self) -> Option<&[bool]> {
        if let FieldValue::MultiSelect { selected, .. } = &self.value {
            Some(selected)
        } else {
            None
        }
    }

    pub fn multi_options(&self) -> Option<&[String]> {
        if let FieldValue::MultiSelect { options, .. } = &self.value {
            Some(options)
        } else {
            None
        }
    }

    pub fn display_value(&self) -> String {
        match &self.value {
            FieldValue::Text(text) => text.clone(),
            FieldValue::Bool(value) => value.to_string(),
            FieldValue::Enum { options, selected } => options
                .get(*selected)
                .cloned()
                .unwrap_or_else(|| "<none>".to_string()),
            FieldValue::MultiSelect { options, selected } => {
                let values = options
                    .iter()
                    .zip(selected.iter())
                    .filter_map(|(option, flag)| if *flag { Some(option.clone()) } else { None })
                    .collect::<Vec<_>>();
                if values.is_empty() {
                    "[]".to_string()
                } else {
                    format!("[{}]", values.join(", "))
                }
            }
            FieldValue::Array(buffer) => format!("[{}]", buffer.trim()),
            FieldValue::Composite(state) => {
                let mut label = state.summary();
                if !state.is_multi() {
                    label.push_str(" (Enter to choose)");
                } else {
                    label.push_str(" (Enter to toggle)");
                }
                label
            }
            FieldValue::CompositeList(state) => {
                let len = state.len();
                if len == 0 {
                    "List: empty (Ctrl+N add)".to_string()
                } else {
                    let selection = state
                        .selected_label()
                        .unwrap_or_else(|| "<no selection>".to_string());
                    format!("List[{len}] • {selection} (Ctrl+Left/Right select, Ctrl+E edit)")
                }
            }
            FieldValue::KeyValue(state) => {
                let len = state.len();
                if len == 0 {
                    "Map: empty (Ctrl+N add)".to_string()
                } else {
                    let selection = state
                        .selected_label()
                        .unwrap_or_else(|| "<no selection>".to_string());
                    format!("Map[{len}] • {selection} (Ctrl+Left/Right select, Ctrl+E edit)")
                }
            }
        }
    }

    pub fn current_value(&self) -> Result<Option<Value>, FieldCoercionError> {
        match (&self.schema.kind, &self.value) {
            (FieldKind::String, FieldValue::Text(text)) => string_value(text, &self.schema),
            (FieldKind::Integer, FieldValue::Text(text)) => integer_value(text, &self.schema),
            (FieldKind::Number, FieldValue::Text(text)) => number_value(text, &self.schema),
            (FieldKind::Json, FieldValue::Text(text)) => string_value(text, &self.schema),
            (FieldKind::Boolean, FieldValue::Bool(value)) => Ok(Some(Value::Bool(*value))),
            (FieldKind::Enum(options), FieldValue::Enum { selected, .. }) => {
                let value = options.get(*selected).cloned().unwrap_or_default();
                Ok(Some(Value::String(value)))
            }
            (FieldKind::Array(_), FieldValue::MultiSelect { options, selected }) => {
                let values = options
                    .iter()
                    .zip(selected.iter())
                    .filter_map(|(option, flag)| {
                        if *flag {
                            Some(Value::String(option.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(Some(Value::Array(values)))
            }
            (FieldKind::Array(inner), FieldValue::Array(buffer)) => {
                array_value(buffer, inner.as_ref(), &self.schema)
            }
            (FieldKind::Composite(_), FieldValue::Composite(state)) => {
                state.build_value(self.schema.required)
            }
            (FieldKind::Array(inner), FieldValue::CompositeList(state))
                if matches!(inner.as_ref(), FieldKind::Composite(_)) =>
            {
                state.build_value()
            }
            (FieldKind::Composite(_), FieldValue::Text(text)) => string_value(text, &self.schema),
            (FieldKind::KeyValue(_), FieldValue::KeyValue(state)) => {
                state.build_value(self.schema.required)
            }
            _ => Ok(None),
        }
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn set_error(&mut self, message: String) {
        self.error = Some(message);
    }

    fn after_edit(&mut self) {
        self.dirty = true;
        self.error = None;
    }
}

fn string_value(contents: &str, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
    if contents.is_empty() && !schema.required {
        return Ok(None);
    }
    Ok(Some(Value::String(contents.to_string())))
}

fn integer_value(
    contents: &str,
    schema: &FieldSchema,
) -> Result<Option<Value>, FieldCoercionError> {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<i64>()
        .map(Value::from)
        .map(Some)
        .map_err(|_| FieldCoercionError {
            pointer: schema.pointer.clone(),
            message: "expected integer".to_string(),
        })
}

fn number_value(contents: &str, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<f64>()
        .map(Value::from)
        .map(Some)
        .map_err(|_| FieldCoercionError {
            pointer: schema.pointer.clone(),
            message: "expected number".to_string(),
        })
}

fn array_value(
    contents: &str,
    inner: &FieldKind,
    schema: &FieldSchema,
) -> Result<Option<Value>, FieldCoercionError> {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        if schema.required {
            return Ok(Some(Value::Array(Vec::new())));
        }
        return Ok(None);
    }

    let mut values = Vec::new();
    for raw in contents.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            continue;
        }
        let value = match inner {
            FieldKind::String => Value::String(item.to_string()),
            FieldKind::Integer => {
                item.parse::<i64>()
                    .map(Value::from)
                    .map_err(|_| FieldCoercionError {
                        pointer: schema.pointer.clone(),
                        message: format!("'{item}' is not a valid integer"),
                    })?
            }
            FieldKind::Number => {
                item.parse::<f64>()
                    .map(Value::from)
                    .map_err(|_| FieldCoercionError {
                        pointer: schema.pointer.clone(),
                        message: format!("'{item}' is not a valid number"),
                    })?
            }
            FieldKind::Boolean => match item.to_ascii_lowercase().as_str() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => {
                    return Err(FieldCoercionError {
                        pointer: schema.pointer.clone(),
                        message: format!("'{item}' is not a valid boolean"),
                    });
                }
            },
            FieldKind::Enum(options) => {
                if options.iter().any(|opt| opt == item) {
                    Value::String(item.to_string())
                } else {
                    return Err(FieldCoercionError {
                        pointer: schema.pointer.clone(),
                        message: format!("value '{item}' is not one of: {}", options.join(", ")),
                    });
                }
            }
            FieldKind::Json | FieldKind::Composite(_) => Value::String(item.to_string()),
            FieldKind::KeyValue(_) => {
                return Err(FieldCoercionError {
                    pointer: schema.pointer.clone(),
                    message: "arrays of key/value maps are not supported".to_string(),
                });
            }
            FieldKind::Array(_) => {
                return Err(FieldCoercionError {
                    pointer: schema.pointer.clone(),
                    message: "nested arrays are not supported".to_string(),
                });
            }
        };
        values.push(value);
    }

    Ok(Some(Value::Array(values)))
}

fn default_text(schema: &FieldSchema) -> String {
    schema
        .default
        .as_ref()
        .map(value_to_string)
        .unwrap_or_default()
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(num) => num.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Array(items) => items
            .iter()
            .map(value_to_string)
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

fn array_to_string(items: &[Value]) -> String {
    items
        .iter()
        .map(value_to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn adjust_numeric_value(buffer: &mut String, kind: &FieldKind, delta: i64) -> bool {
    match kind {
        FieldKind::Integer => {
            let current = buffer.trim().parse::<i64>().unwrap_or(0);
            let next = current.saturating_add(delta);
            *buffer = next.to_string();
            true
        }
        FieldKind::Number => {
            let current = buffer.trim().parse::<f64>().unwrap_or(0.0);
            *buffer = (current + delta as f64).to_string();
            true
        }
        _ => false,
    }
}
