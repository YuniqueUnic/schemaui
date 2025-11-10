use std::cell::{RefCell, RefMut};

use serde_json::Value;

use crate::domain::{parse_form_schema, CompositeField, CompositeMode};

use super::{error::FieldCoercionError, state::FormState};

#[derive(Debug, Clone)]
pub struct CompositeState {
    pointer: String,
    mode: CompositeMode,
    variants: Vec<CompositeVariantState>,
}

#[derive(Debug, Clone)]
pub struct CompositeVariantState {
    #[allow(dead_code)]
    id: String,
    title: String,
    #[allow(dead_code)]
    description: Option<String>,
    schema: Value,
    active: bool,
    form: RefCell<Option<FormState>>,
}

#[derive(Debug)]
pub struct CompositeEditorSession {
    pub variant_index: usize,
    pub title: String,
    pub description: Option<String>,
    pub form_state: FormState,
}

#[derive(Debug, Clone)]
pub struct CompositeVariantSummary {
    pub title: String,
    pub description: Option<String>,
    pub lines: Vec<String>,
}

impl CompositeState {
    pub fn new(pointer: &str, field: &CompositeField) -> Self {
        let mut variants = Vec::with_capacity(field.variants.len());
        for (index, variant) in field.variants.iter().enumerate() {
            variants.push(CompositeVariantState {
                id: variant.id.clone(),
                title: variant.title.clone(),
                description: variant.description.clone(),
                schema: variant.schema.clone(),
                active: matches!(field.mode, CompositeMode::OneOf) && index == 0,
                form: RefCell::new(None),
            });
        }

        Self {
            pointer: pointer.to_string(),
            mode: field.mode.clone(),
            variants,
        }
    }

    pub fn summary(&self) -> String {
        match self.mode {
            CompositeMode::OneOf => self
                .variants
                .iter()
                .find(|variant| variant.active)
                .map(|variant| format!("Variant: {}", variant.title))
                .unwrap_or_else(|| "Variant: <none>".to_string()),
            CompositeMode::AnyOf => {
                let active = self
                    .variants
                    .iter()
                    .filter(|variant| variant.active)
                    .map(|variant| variant.title.clone())
                    .collect::<Vec<_>>();
                if active.is_empty() {
                    "Variants: []".to_string()
                } else {
                    format!("Variants: {}", active.join(", "))
                }
            }
        }
    }

    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    pub fn is_multi(&self) -> bool {
        matches!(self.mode, CompositeMode::AnyOf)
    }

    pub fn variant_count(&self) -> usize {
        self.variants.len()
    }

    pub fn active_summaries(&self) -> Vec<CompositeVariantSummary> {
        let mut summaries = Vec::new();
        for variant in self.variants.iter().filter(|variant| variant.active) {
            match variant.snapshot(self.pointer()) {
                Ok(summary) => summaries.push(summary),
                Err(err) => summaries.push(CompositeVariantSummary {
                    title: variant.title.clone(),
                    description: variant.description.clone(),
                    lines: vec![format!("Error: {}", err.message)],
                }),
            }
        }
        summaries
    }

    pub fn option_titles(&self) -> Vec<String> {
        self.variants.iter().map(|variant| variant.title.clone()).collect()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.variants.iter().position(|variant| variant.active)
    }

    pub fn active_flags(&self) -> Vec<bool> {
        self.variants.iter().map(|variant| variant.active).collect()
    }

    pub fn active_indices(&self) -> Vec<usize> {
        self.variants
            .iter()
            .enumerate()
            .filter_map(|(idx, variant)| if variant.active { Some(idx) } else { None })
            .collect()
    }

    pub fn apply_single(&mut self, index: usize) -> bool {
        if !matches!(self.mode, CompositeMode::OneOf) {
            return false;
        }
        if self.variants.is_empty() {
            return false;
        }
        let target = index.min(self.variants.len() - 1);
        let mut changed = false;
        for (idx, variant) in self.variants.iter_mut().enumerate() {
            let next_state = idx == target;
            if variant.active != next_state {
                variant.active = next_state;
                changed = true;
            }
        }
        changed
    }

    pub fn take_editor_session(
        &self,
        pointer: &str,
        variant_index: usize,
    ) -> Result<CompositeEditorSession, FieldCoercionError> {
        let variant = self
            .variants
            .get(variant_index)
            .ok_or_else(|| FieldCoercionError {
                pointer: pointer.to_string(),
                message: "invalid variant selection".to_string(),
            })?;
        if !variant.active {
            return Err(FieldCoercionError {
                pointer: pointer.to_string(),
                message: "variant is not active; select it before editing".to_string(),
            });
        }
        let form_state = variant.take_form(pointer)?;
        Ok(CompositeEditorSession {
            variant_index,
            title: variant.title.clone(),
            description: variant.description.clone(),
            form_state,
        })
    }

    pub fn restore_editor_session(&self, session: CompositeEditorSession) {
        if let Some(variant) = self.variants.get(session.variant_index) {
            variant.store_form(session.form_state);
        }
    }

    pub fn apply_multi(&mut self, flags: &[bool]) -> bool {
        if !matches!(self.mode, CompositeMode::AnyOf) {
            return false;
        }
        if flags.len() != self.variants.len() {
            return false;
        }
        let mut changed = false;
        for (variant, flag) in self.variants.iter_mut().zip(flags.iter()) {
            if variant.active != *flag {
                variant.active = *flag;
                changed = true;
            }
        }
        changed
    }

    pub fn build_value(&self, required: bool) -> Result<Option<Value>, FieldCoercionError> {
        match self.mode {
            CompositeMode::OneOf => {
                if let Some(variant) = self.variants.iter().find(|variant| variant.active) {
                    let form = variant.borrow_form(self.pointer())?;
                    match form.try_build_value() {
                        Ok(value) => Ok(Some(value)),
                        Err(mut err) => {
                            err.pointer = join_pointer(self.pointer(), &err.pointer);
                            Err(err)
                        }
                    }
                } else if required {
                    Err(FieldCoercionError {
                        pointer: self.pointer.clone(),
                        message: "oneOf requires a selected variant".to_string(),
                    })
                } else {
                    Ok(None)
                }
            }
            CompositeMode::AnyOf => {
                let mut values = Vec::new();
                for variant in self.variants.iter().filter(|variant| variant.active) {
                    let form = variant.borrow_form(self.pointer())?;
                    match form.try_build_value() {
                        Ok(value) => values.push(value),
                        Err(mut err) => {
                            err.pointer = join_pointer(self.pointer(), &err.pointer);
                            return Err(err);
                        }
                    }
                }

                if values.is_empty() {
                    if required {
                        Err(FieldCoercionError {
                            pointer: self.pointer.clone(),
                            message: "anyOf requires at least one active variant".to_string(),
                        })
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(Some(Value::Array(values)))
                }
            }
        }
    }
}

impl CompositeVariantState {
    fn ensure_form_ready(&self, pointer: &str) -> Result<(), FieldCoercionError> {
        if self.form.borrow().is_some() {
            return Ok(());
        }
        let schema = parse_form_schema(&self.schema).map_err(|err| FieldCoercionError {
            pointer: pointer.to_string(),
            message: format!(
                "failed to parse composite variant '{}': {err}",
                self.title
            ),
        })?;
        *self.form.borrow_mut() = Some(FormState::from_schema(&schema));
        Ok(())
    }

    fn borrow_form(&self, pointer: &str) -> Result<RefMut<'_, FormState>, FieldCoercionError> {
        self.ensure_form_ready(pointer)?;
        Ok(RefMut::map(self.form.borrow_mut(), |slot| {
            slot.as_mut().expect("variant form should be initialized")
        }))
    }

    fn take_form(&self, pointer: &str) -> Result<FormState, FieldCoercionError> {
        self.ensure_form_ready(pointer)?;
        Ok(self
            .form
            .borrow_mut()
            .take()
            .expect("variant form should be initialized"))
    }

    fn store_form(&self, form_state: FormState) {
        *self.form.borrow_mut() = Some(form_state);
    }

    fn snapshot(
        &self,
        pointer: &str,
    ) -> Result<CompositeVariantSummary, FieldCoercionError> {
        let form = self.borrow_form(pointer)?;
        let mut lines = Vec::new();
        if form.sections.is_empty() {
            lines.push("No fields defined for this variant.".to_string());
        } else {
            for section in &form.sections {
                lines.push(format!("Section: {}", section.title));
                if section.fields.is_empty() {
                    lines.push("  • <empty>".to_string());
                } else {
                    for field in &section.fields {
                        lines.push(format!(
                            "  • {} = {}",
                            field.schema.display_label(),
                            field.display_value()
                        ));
                    }
                }
            }
        }
        Ok(CompositeVariantSummary {
            title: self.title.clone(),
            description: self.description.clone(),
            lines,
        })
    }
}

fn join_pointer(base: &str, child: &str) -> String {
    match (base.is_empty(), child.is_empty()) {
        (true, true) => String::new(),
        (true, false) => child.to_string(),
        (false, true) => base.to_string(),
        (false, false) => {
            if child.starts_with('/') {
                format!("{base}{child}")
            } else if base.ends_with('/') {
                format!("{base}{child}")
            } else {
                format!("{base}/{child}")
            }
        }
    }
}
