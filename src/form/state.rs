use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Map, Value};

use crate::domain::{FieldKind, FieldSchema, FormSchema, FormSection};

#[derive(Debug, Clone)]
pub struct SectionState {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub fields: Vec<FieldState>,
}

#[derive(Debug, Clone)]
pub struct FieldState {
    pub schema: FieldSchema,
    pub value: FieldValue,
    pub dirty: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Text(String),
    Bool(bool),
    Enum {
        options: Vec<String>,
        selected: usize,
    },
    Array(String),
}

#[derive(Debug, Clone)]
pub struct FormState {
    pub sections: Vec<SectionState>,
    pub section_index: usize,
    pub field_index: usize,
}

#[derive(Debug, Clone)]
pub struct FieldCoercionError {
    pub pointer: String,
    pub message: String,
}

impl std::fmt::Display for FieldCoercionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.pointer, self.message)
    }
}

impl std::error::Error for FieldCoercionError {}

impl FormState {
    pub fn from_schema(schema: &FormSchema) -> Self {
        let sections = if schema.sections.is_empty() {
            vec![empty_section()]
        } else {
            schema.sections.iter().map(SectionState::from).collect()
        };

        FormState {
            sections,
            section_index: 0,
            field_index: 0,
        }
    }

    pub fn focused_field_mut(&mut self) -> Option<&mut FieldState> {
        self.sections
            .get_mut(self.section_index)
            .and_then(|section| section.fields.get_mut(self.field_index))
    }

    pub fn focused_field(&self) -> Option<&FieldState> {
        self.sections
            .get(self.section_index)
            .and_then(|section| section.fields.get(self.field_index))
    }

    pub fn focus_next_field(&mut self) {
        if self.sections.is_empty() {
            return;
        }

        if self.sections[self.section_index].fields.is_empty() {
            return;
        }

        if self.field_index + 1 < self.sections[self.section_index].fields.len() {
            self.field_index += 1;
        } else {
            self.section_index = (self.section_index + 1) % self.sections.len();
            self.field_index = 0;
        }
    }

    pub fn focus_prev_field(&mut self) {
        if self.sections.is_empty() {
            return;
        }

        if self.sections[self.section_index].fields.is_empty() {
            return;
        }

        if self.field_index > 0 {
            self.field_index -= 1;
        } else {
            if self.section_index == 0 {
                self.section_index = self.sections.len() - 1;
            } else {
                self.section_index -= 1;
            }
            if let Some(section) = self.sections.get(self.section_index) {
                if !section.fields.is_empty() {
                    self.field_index = section.fields.len() - 1;
                }
            }
        }
    }

    pub fn focus_next_section(&mut self, direction: i32) {
        if self.sections.is_empty() {
            return;
        }

        let len = self.sections.len();
        let mut index = self.section_index as i32 + direction;
        if index < 0 {
            index = len as i32 - 1;
        }
        if index as usize >= len {
            index = 0;
        }
        self.section_index = index as usize;
        self.field_index = self.field_index.min(
            self.sections[self.section_index]
                .fields
                .len()
                .saturating_sub(1),
        );
    }

    pub fn try_build_value(&self) -> Result<Value, FieldCoercionError> {
        let mut root = Value::Object(Map::new());
        for section in &self.sections {
            for field in &section.fields {
                if let Some(value) = field.current_value()? {
                    insert_path(&mut root, &field.schema.path, value);
                }
            }
        }
        Ok(root)
    }

    pub fn clear_errors(&mut self) {
        for section in &mut self.sections {
            for field in &mut section.fields {
                field.error = None;
            }
        }
    }

    pub fn set_error(&mut self, pointer: &str, message: String) -> bool {
        for section in &mut self.sections {
            for field in &mut section.fields {
                if field.schema.pointer == pointer {
                    field.error = Some(message);
                    return true;
                }
            }
        }
        false
    }

    pub fn field_mut_by_pointer(&mut self, pointer: &str) -> Option<&mut FieldState> {
        for section in &mut self.sections {
            for field in &mut section.fields {
                if field.schema.pointer == pointer {
                    return Some(field);
                }
            }
        }
        None
    }

    pub fn is_dirty(&self) -> bool {
        self.sections
            .iter()
            .any(|section| section.fields.iter().any(|field| field.dirty))
    }
}

impl SectionState {
    fn from(section: &FormSection) -> Self {
        let fields = section
            .fields
            .iter()
            .cloned()
            .map(FieldState::from_schema)
            .collect();

        SectionState {
            id: section.id.clone(),
            title: section.title.clone(),
            description: section.description.clone(),
            fields,
        }
    }
}

impl FieldState {
    fn from_schema(schema: FieldSchema) -> Self {
        let value = match &schema.kind {
            FieldKind::String | FieldKind::Integer | FieldKind::Number => {
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
            FieldKind::Array(_) => {
                let default = schema
                    .default
                    .as_ref()
                    .and_then(|value| value.as_array())
                    .map(|values| array_to_string(values))
                    .unwrap_or_default();
                FieldValue::Array(default)
            }
        };

        FieldState {
            schema,
            value,
            dirty: false,
            error: None,
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> bool {
        match &mut self.value {
            FieldValue::Text(buffer) | FieldValue::Array(buffer) => match key.code {
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
                KeyCode::Char(' ') => {
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

    pub fn display_value(&self) -> String {
        match &self.value {
            FieldValue::Text(text) => text.clone(),
            FieldValue::Bool(value) => value.to_string(),
            FieldValue::Enum { options, selected } => options
                .get(*selected)
                .cloned()
                .unwrap_or_else(|| "<none>".to_string()),
            FieldValue::Array(buffer) => format!("[{}]", buffer.trim()),
        }
    }

    fn current_value(&self) -> Result<Option<Value>, FieldCoercionError> {
        match (&self.schema.kind, &self.value) {
            (FieldKind::String, FieldValue::Text(text)) => string_value(text, &self.schema),
            (FieldKind::Integer, FieldValue::Text(text)) => integer_value(text, &self.schema),
            (FieldKind::Number, FieldValue::Text(text)) => number_value(text, &self.schema),
            (FieldKind::Boolean, FieldValue::Bool(value)) => Ok(Some(Value::Bool(*value))),
            (FieldKind::Enum(options), FieldValue::Enum { selected, .. }) => {
                let value = options.get(*selected).cloned().unwrap_or_default();
                Ok(Some(Value::String(value)))
            }
            (FieldKind::Array(inner), FieldValue::Array(buffer)) => {
                array_value(buffer, inner.as_ref(), &self.schema)
            }
            _ => Ok(None),
        }
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

fn insert_path(root: &mut Value, path: &[String], value: Value) {
    if path.is_empty() {
        *root = value;
        return;
    }

    if !root.is_object() {
        *root = Value::Object(Map::new());
    }

    if let Value::Object(obj) = root {
        if path.len() == 1 {
            obj.insert(path[0].clone(), value);
            return;
        }

        let entry = obj
            .entry(path[0].clone())
            .or_insert_with(|| Value::Object(Map::new()));
        insert_path(entry, &path[1..], value);
    }
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

fn empty_section() -> SectionState {
    SectionState {
        id: "general".to_string(),
        title: "General".to_string(),
        description: None,
        fields: Vec::new(),
    }
}
