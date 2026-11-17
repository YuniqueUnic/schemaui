use serde_json::{Map, Value};

use super::{FieldCoercionError, FormState};

impl FormState {
    pub fn try_build_value(&self) -> Result<Value, FieldCoercionError> {
        let mut document = self
            .document
            .clone()
            .unwrap_or_else(|| Value::Object(Map::new()));
        for field in self.iter_sections().flat_map(|section| &section.fields) {
            if self.document.is_some() && !field.edited {
                continue;
            }
            match field.current_value()? {
                Some(value) => insert_path(&mut document, &field.schema.path, value),
                None => remove_path(&mut document, &field.schema.path),
            }
        }
        Ok(document)
    }

    pub fn seed_from_value(&mut self, value: &Value) {
        self.document = Some(value.clone());
        for field in self
            .iter_sections_mut()
            .flat_map(|section| &mut section.fields)
        {
            if let Some(value) = value_at_path(value, &field.schema.path) {
                field.seed_value(value);
            } else {
                field.reset_value();
            }
        }
        self.normalize_focus();
    }

    pub fn initialize_document(&mut self, value: &Value) -> Result<(), FieldCoercionError> {
        let mut document = value.clone();
        for section in self.iter_sections() {
            if !section.path.is_empty()
                && section.required
                && value_at_path(&document, &section.path).is_none()
                && parent_is_object(&document, &section.path)
            {
                insert_path(
                    &mut document,
                    &section.path,
                    section
                        .default
                        .clone()
                        .unwrap_or_else(|| Value::Object(Map::new())),
                );
            }
            for field in &section.fields {
                if !field.schema.required
                    || value_at_path(&document, &field.schema.path).is_some()
                    || !parent_is_object(&document, &field.schema.path)
                {
                    continue;
                }
                let default = match &field.schema.default {
                    Some(value) => Some(value.clone()),
                    None => field.current_value()?,
                };
                if let Some(value) = default {
                    insert_path(&mut document, &field.schema.path, value);
                }
            }
        }
        self.seed_from_value(&document);
        Ok(())
    }

    pub(crate) fn value_at_field_path(
        &self,
        path: &[String],
    ) -> Result<Option<Value>, FieldCoercionError> {
        let mut value = self
            .document
            .as_ref()
            .and_then(|document| value_at_path(document, path))
            .cloned();
        for field in self.iter_sections().flat_map(|section| &section.fields) {
            if self.document.is_some() && !field.edited {
                continue;
            }
            let field_path = &field.schema.path;
            if path.starts_with(field_path) {
                value = field
                    .current_value()?
                    .and_then(|value| value_at_path(&value, &path[field_path.len()..]).cloned());
            } else if field_path.starts_with(path) {
                let relative = &field_path[path.len()..];
                match field.current_value()? {
                    Some(current) => insert_path(
                        value.get_or_insert_with(|| Value::Object(Map::new())),
                        relative,
                        current,
                    ),
                    None => {
                        if let Some(value) = &mut value {
                            remove_path(value, relative);
                        }
                    }
                }
            }
        }
        Ok(value)
    }
}

fn insert_path(root: &mut Value, path: &[String], value: Value) {
    let Some((first, remaining)) = path.split_first() else {
        *root = value;
        return;
    };
    if !root.is_object() {
        *root = Value::Object(Map::new());
    }
    if let Some(object) = root.as_object_mut() {
        if remaining.is_empty() {
            object.insert(first.clone(), value);
        } else {
            insert_path(
                object
                    .entry(first.clone())
                    .or_insert_with(|| Value::Object(Map::new())),
                remaining,
                value,
            );
        }
    }
}

fn remove_path(root: &mut Value, path: &[String]) {
    let Some((key, parent)) = path.split_last() else {
        return;
    };
    let mut current = root;
    for segment in parent {
        let Some(value) = current.get_mut(segment) else {
            return;
        };
        current = value;
    }
    if let Some(object) = current.as_object_mut() {
        object.remove(key);
    }
}

fn parent_is_object(root: &Value, path: &[String]) -> bool {
    path.split_last()
        .and_then(|(_, parent)| value_at_path(root, parent))
        .is_some_and(Value::is_object)
}

fn value_at_path<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}
