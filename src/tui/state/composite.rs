use std::sync::Arc;

use serde_json::Value;

use crate::tui::model::{CompositeField, CompositeMode, CompositeVariantLayout};

use super::{
    error::FieldCoercionError,
    field::components::ComponentPalette,
    value_summary::{summarize_inline_value, summarize_inline_value_with_limit},
};

mod composite_list;
mod variant;

pub use composite_list::{CompositeListEditorContext, CompositeListState};
use variant::CompositeVariantState;

#[derive(Debug, Clone)]
pub struct CompositeState {
    pointer: String,
    mode: CompositeMode,
    variants: Vec<CompositeVariantState>,
    value: Option<Value>,
    seed_error: Option<FieldCoercionError>,
}

#[derive(Debug, Clone)]
pub struct CompositeEditorSession {
    pub variant_index: usize,
    pub title: String,
    pub description: Option<String>,
    pub form_state: super::FormState,
    pub schema: Value,
    layout: CompositeVariantLayout,
    pending_edits: bool,
}

impl CompositeEditorSession {
    pub fn value(&self) -> Result<Value, FieldCoercionError> {
        self.layout.unwrap_value(self.form_state.try_build_value()?)
    }

    pub(crate) fn seed_draft(&mut self, value: &Value, pending_edits: bool) {
        self.form_state.seed_from_value(&self.layout.payload(value));
        self.pending_edits = pending_edits;
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.pending_edits || self.form_state.is_dirty()
    }

    pub(crate) fn mark_clean(&mut self) {
        self.pending_edits = false;
        self.form_state.mark_clean();
    }
}

#[derive(Debug, Clone)]
pub struct CompositeVariantSummary {
    pub title: String,
    pub description: Option<String>,
    pub lines: Vec<String>,
}

impl CompositeState {
    pub fn new(pointer: &str, field: &CompositeField, palette: Arc<ComponentPalette>) -> Self {
        let variants = field
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                CompositeVariantState::new(
                    variant,
                    matches!(field.mode, CompositeMode::OneOf) && index == 0,
                    Arc::clone(&palette),
                )
            })
            .collect();
        Self {
            pointer: pointer.to_string(),
            mode: field.mode.clone(),
            variants,
            value: None,
            seed_error: None,
        }
    }

    pub fn summary(&self) -> String {
        match self.mode {
            CompositeMode::OneOf => self
                .variants
                .iter()
                .enumerate()
                .find(|(_, variant)| variant.active)
                .map(|(idx, variant)| format!("Variant: #{} {}", idx + 1, variant.title))
                .unwrap_or_else(|| "Variant: <none; select a matching variant>".to_string()),
            CompositeMode::AnyOf => {
                let active = self
                    .variants
                    .iter()
                    .enumerate()
                    .filter(|(_, variant)| variant.active)
                    .map(|(idx, variant)| format!("#{} {}", idx + 1, variant.title))
                    .collect::<Vec<_>>();
                if active.is_empty() {
                    "Variants: []".to_string()
                } else {
                    format!("Variants: {}", active.join(", "))
                }
            }
        }
    }

    pub fn summary_with_preview(&self) -> String {
        let summary = self.summary();
        match self.build_value(false) {
            Ok(Some(value)) => format!("{summary} {}", summarize_inline_value(&value)),
            Ok(None) => summary,
            Err(error) => format!("{summary} Error: {}", error.message),
        }
    }

    pub fn summary_with_preview_limit(&self, max_visible: usize) -> String {
        use super::value_summary::truncate_visible_text;
        use unicode_width::UnicodeWidthStr;

        if max_visible == 0 {
            return String::new();
        }
        let summary = self.summary();
        let summary_width = UnicodeWidthStr::width(summary.as_str());
        if summary_width >= max_visible {
            return truncate_visible_text(&summary, max_visible);
        }
        let preview_budget = max_visible.saturating_sub(summary_width + 1);
        let preview = match self.build_value(false) {
            Ok(Some(value)) => summarize_inline_value_with_limit(&value, preview_budget),
            Ok(None) => String::new(),
            Err(error) => {
                truncate_visible_text(&format!("Error: {}", error.message), preview_budget)
            }
        };
        if preview.is_empty() {
            summary
        } else {
            format!("{summary} {preview}")
        }
    }

    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    pub fn rebind_pointer(&mut self, pointer: &str) {
        self.pointer = pointer.to_string();
    }

    pub fn seed_from_value(&mut self, value: &Value) -> Result<(), FieldCoercionError> {
        self.value = Some(value.clone());
        self.seed_error = None;
        let matches = self
            .variants
            .iter()
            .map(|variant| {
                jsonschema::validator_for(&variant.schema)
                    .map(|validator| validator.is_valid(value))
                    .map_err(|error| FieldCoercionError {
                        pointer: self.pointer.clone(),
                        message: format!("invalid variant '{}': {error}", variant.title),
                    })
            })
            .collect::<Result<Vec<_>, _>>();
        match matches {
            Ok(flags) => {
                let unique = flags.iter().filter(|flag| **flag).count() == 1;
                let multi = self.is_multi();
                let single_variant = self.variants.len() == 1;
                for (variant, matched) in self.variants.iter_mut().zip(flags) {
                    variant.active = single_variant || (matched && (multi || unique));
                }
                Ok(())
            }
            Err(error) => {
                for variant in &mut self.variants {
                    variant.active = false;
                }
                self.seed_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn is_multi(&self) -> bool {
        matches!(self.mode, CompositeMode::AnyOf)
    }

    pub fn active_summaries(&self) -> Vec<CompositeVariantSummary> {
        self.variants
            .iter()
            .enumerate()
            .filter(|(_, variant)| variant.active)
            .map(|(index, variant)| {
                let mut summary = match variant.snapshot(self.pointer(), self.value.as_ref()) {
                    Ok(summary) => summary,
                    Err(error) => CompositeVariantSummary {
                        title: variant.title.clone(),
                        description: variant.description.clone(),
                        lines: vec![format!("Error: {}", error.message)],
                    },
                };
                summary.title = format!("#{} {}", index + 1, summary.title);
                summary
            })
            .collect()
    }

    pub fn option_titles(&self) -> Vec<String> {
        self.variants
            .iter()
            .map(|variant| variant.title.clone())
            .collect()
    }

    pub fn variant_descriptions(&self) -> Vec<Option<String>> {
        self.variants
            .iter()
            .map(|variant| variant.description.clone())
            .collect()
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
            .filter_map(|(index, variant)| variant.active.then_some(index))
            .collect()
    }

    pub fn apply_single(&mut self, index: usize) -> bool {
        if self.is_multi() || index >= self.variants.len() || self.selected_index() == Some(index) {
            return false;
        }
        let replacing_variant = self.selected_index().is_some();
        for (current, variant) in self.variants.iter_mut().enumerate() {
            variant.active = current == index;
        }
        if replacing_variant {
            self.value = None;
        }
        self.seed_error = None;
        true
    }

    pub fn rotate_single(&mut self, delta: i32) -> bool {
        if self.is_multi() || self.variants.is_empty() {
            return false;
        }
        let index = self.selected_index().map_or_else(
            || {
                if delta < 0 {
                    self.variants.len() - 1
                } else {
                    0
                }
            },
            |current| {
                (current as i64 + i64::from(delta)).rem_euclid(self.variants.len() as i64) as usize
            },
        );
        self.apply_single(index)
    }

    pub fn ensure_editable_variant(&mut self) {
        self.ensure_editable_variant_with_index(None);
    }

    pub fn ensure_editable_variant_with_index(&mut self, variant_index: Option<usize>) {
        let target = match variant_index {
            Some(index) if index < self.variants.len() => index,
            Some(_) => return,
            None if self.value.is_none()
                && self.selected_index().is_none()
                && !self.variants.is_empty() =>
            {
                0
            }
            None => return,
        };
        if self.is_multi() {
            for (index, variant) in self.variants.iter_mut().enumerate() {
                variant.active = index == target;
            }
        } else {
            self.apply_single(target);
        }
    }

    pub fn open_editor_session(
        &self,
        pointer: &str,
        variant_index: usize,
    ) -> Result<CompositeEditorSession, FieldCoercionError> {
        if let Some(error) = &self.seed_error {
            return Err(error.clone());
        }
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
        variant.editor_session(pointer, variant_index, self.value.as_ref())
    }

    pub fn apply_editor_session(
        &mut self,
        session: CompositeEditorSession,
    ) -> Result<bool, FieldCoercionError> {
        if session.variant_index >= self.variants.len() {
            return Err(FieldCoercionError {
                pointer: self.pointer.clone(),
                message: "invalid variant selection".to_string(),
            });
        }
        let value = session.value()?;
        let changed = self.build_value(false) != Ok(Some(value.clone()));
        self.value = Some(value);
        self.seed_error = None;
        Ok(changed)
    }

    pub fn apply_multi(&mut self, flags: &[bool]) -> bool {
        if !self.is_multi() || flags.len() != self.variants.len() {
            return false;
        }
        let mut changed = false;
        for (variant, flag) in self.variants.iter_mut().zip(flags) {
            changed |= variant.active != *flag;
            variant.active = *flag;
        }
        changed
    }

    pub fn build_value(&self, required: bool) -> Result<Option<Value>, FieldCoercionError> {
        if let Some(error) = &self.seed_error {
            return Err(error.clone());
        }
        if let Some(value) = &self.value {
            return Ok(Some(value.clone()));
        }
        if let Some((index, variant)) = self
            .variants
            .iter()
            .enumerate()
            .find(|(_, variant)| variant.active)
        {
            return variant
                .editor_session(self.pointer(), index, None)?
                .value()
                .map(Some);
        }
        if required {
            Err(FieldCoercionError {
                pointer: self.pointer.clone(),
                message: "select a composite variant before saving".to_string(),
            })
        } else {
            Ok(None)
        }
    }
}
