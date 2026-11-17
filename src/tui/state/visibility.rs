use serde_json::Value;

use crate::ui_ast::{VisibleWhen, VisibleWhenOp};

use super::{FieldState, FormState, SectionState};

impl FormState {
    pub fn field_is_visible(&self, field: &FieldState) -> bool {
        self.path_is_visible(&field.schema.path)
            && self.rule_matches(
                &field.schema.path,
                field.schema.presentation.visible_when.as_ref(),
            )
    }

    pub fn section_is_visible(&self, section: &SectionState) -> bool {
        self.path_is_visible(&section.path)
    }

    pub fn visible_field_indices(&self) -> Vec<usize> {
        self.active_section()
            .map(|section| {
                section
                    .fields
                    .iter()
                    .enumerate()
                    .filter_map(|(index, field)| self.field_is_visible(field).then_some(index))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn visible_root_indices(&self) -> Vec<usize> {
        self.roots
            .iter()
            .enumerate()
            .filter_map(|(index, root)| {
                root.sections
                    .iter()
                    .any(|section| self.section_is_visible(section))
                    .then_some(index)
            })
            .collect()
    }

    pub fn visible_section_indices(&self) -> Vec<usize> {
        self.roots
            .get(self.root_index())
            .map(|root| {
                root.sections
                    .iter()
                    .enumerate()
                    .filter_map(|(index, section)| {
                        self.section_is_visible(section).then_some(index)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn path_is_visible(&self, path: &[String]) -> bool {
        self.roots
            .iter()
            .flat_map(|root| &root.sections)
            .filter(|section| !section.path.is_empty() && path.starts_with(&section.path))
            .all(|section| {
                self.rule_matches(&section.path, section.presentation.visible_when.as_ref())
            })
    }

    fn rule_matches(&self, path: &[String], rule: Option<&VisibleWhen>) -> bool {
        let Some(rule) = rule else {
            return true;
        };
        let mut sibling = path[..path.len().saturating_sub(1)].to_vec();
        sibling.push(rule.field.clone());
        let current = match self.value_at_field_path(&sibling) {
            Ok(value) => value,
            // An invalid controller must not conceal fields or their validation errors.
            Err(_) => return true,
        };
        match rule.op {
            VisibleWhenOp::Equals => current.as_ref() == Some(&rule.value),
            VisibleWhenOp::Contains => current
                .as_ref()
                .and_then(Value::as_array)
                .is_some_and(|items| items.contains(&rule.value)),
        }
    }
}
