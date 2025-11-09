use serde_json::{Map, Value};

use crate::domain::FormSchema;

use super::{error::FieldCoercionError, field::FieldState, section::SectionState};

#[derive(Debug, Clone)]
pub struct FormState {
    pub sections: Vec<SectionState>,
    pub section_index: usize,
    pub field_index: usize,
}

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
                field.clear_error();
            }
        }
    }

    pub fn set_error(&mut self, pointer: &str, message: String) -> bool {
        for section in &mut self.sections {
            for field in &mut section.fields {
                if field.schema.pointer == pointer {
                    field.set_error(message.clone());
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

fn empty_section() -> SectionState {
    SectionState {
        id: "general".to_string(),
        title: "General".to_string(),
        description: None,
        fields: Vec::new(),
    }
}
