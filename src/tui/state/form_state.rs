use std::sync::Arc;

use serde_json::Value;

use crate::tui::model::FormSchema;
use crate::tui::state::LayoutNavModel;

use super::{
    error::FieldCoercionError,
    field::{FieldState, components::ComponentPalette},
    section::SectionState,
    ui_store::UiStores,
};

mod document;
mod layout_focus;
mod navigation;

#[derive(Debug, Clone)]
pub struct RootSectionState {
    #[allow(dead_code)]
    pub id: String,
    pub title: String,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub sections: Vec<SectionState>,
}

#[derive(Debug, Clone)]
pub struct FormState {
    pub roots: Vec<RootSectionState>,
    ui: UiStores,
    layout_nav: Option<LayoutNavModel>,
    document: Option<Value>,
}

impl FormState {
    #[allow(dead_code)]
    pub fn from_schema(schema: &FormSchema) -> Self {
        Self::from_schema_with_palette(schema, Arc::new(ComponentPalette::default()))
    }

    pub fn from_schema_with_palette(schema: &FormSchema, palette: Arc<ComponentPalette>) -> Self {
        let mut roots = Vec::new();
        if schema.roots.is_empty() {
            roots.push(empty_root_state());
        } else {
            for root in &schema.roots {
                let mut sections = Vec::new();
                for section in &root.sections {
                    SectionState::collect(section, 0, &palette, &mut sections);
                }
                if sections.is_empty() {
                    sections.push(empty_section_state());
                }
                roots.push(RootSectionState {
                    id: root.id.clone(),
                    title: root.title.clone(),
                    description: root.description.clone(),
                    sections,
                });
            }
        }
        let mut state = Self {
            roots,
            ui: UiStores::new(),
            layout_nav: None,
            document: None,
        };
        state.normalize_focus();
        state
    }

    pub fn from_sections(
        root_id: impl Into<String>,
        root_title: impl Into<String>,
        root_description: Option<String>,
        sections: Vec<SectionState>,
    ) -> Self {
        let mut state = Self {
            roots: vec![RootSectionState {
                id: root_id.into(),
                title: root_title.into(),
                description: root_description,
                sections,
            }],
            ui: UiStores::new(),
            layout_nav: None,
            document: None,
        };
        state.normalize_focus();
        state
    }

    #[cfg(all(test, feature = "json"))]
    pub(crate) fn from_roots_for_test(roots: Vec<RootSectionState>) -> Self {
        let mut state = Self {
            roots,
            ui: UiStores::new(),
            layout_nav: None,
            document: None,
        };
        state.normalize_focus();
        state
    }

    pub fn with_layout_nav(mut self, layout_nav: LayoutNavModel) -> Self {
        self.layout_nav = Some(layout_nav);
        self
    }

    pub fn set_layout_nav(&mut self, layout_nav: LayoutNavModel) {
        self.layout_nav = Some(layout_nav);
    }

    pub fn layout_nav(&self) -> Option<&LayoutNavModel> {
        self.layout_nav.as_ref()
    }

    pub fn clear_errors(&mut self) {
        for section in self.iter_sections_mut() {
            for field in &mut section.fields {
                field.clear_error();
            }
        }
    }

    pub fn mark_clean(&mut self) {
        for section in self.iter_sections_mut() {
            for field in &mut section.fields {
                field.dirty = false;
            }
        }
    }

    pub fn set_error(&mut self, pointer: &str, message: String) -> bool {
        for section in self.iter_sections_mut() {
            for field in &mut section.fields {
                if field.schema.pointer == pointer {
                    field.set_error(message.clone());
                    return true;
                }
            }
        }
        false
    }

    pub fn clear_error(&mut self, pointer: &str) {
        for section in self.iter_sections_mut() {
            for field in &mut section.fields {
                if field.schema.pointer == pointer {
                    field.clear_error();
                }
            }
        }
    }

    pub fn field_mut_by_pointer(&mut self, pointer: &str) -> Option<&mut FieldState> {
        for section in self.iter_sections_mut() {
            for field in &mut section.fields {
                if field.schema.pointer == pointer {
                    return Some(field);
                }
            }
        }
        None
    }

    pub fn field_by_pointer(&self, pointer: &str) -> Option<&FieldState> {
        for section in self.iter_sections() {
            for field in &section.fields {
                if field.schema.pointer == pointer {
                    return Some(field);
                }
            }
        }
        None
    }

    pub fn is_dirty(&self) -> bool {
        self.iter_sections()
            .any(|section| section.fields.iter().any(|field| field.dirty))
    }

    pub fn error_count(&self) -> usize {
        self.iter_sections()
            .map(|section| {
                section
                    .fields
                    .iter()
                    .filter(|field| field.error.is_some())
                    .count()
            })
            .sum()
    }

    pub fn error_entries(&self) -> Vec<(String, String)> {
        let mut entries = Vec::new();
        for section in self.iter_sections() {
            for field in &section.fields {
                if let Some(err) = &field.error {
                    entries.push((field.schema.pointer.clone(), err.clone()));
                }
            }
        }
        entries
    }

    fn iter_sections(&self) -> impl Iterator<Item = &SectionState> {
        self.roots.iter().flat_map(|root| root.sections.iter())
    }

    fn iter_sections_mut(&mut self) -> impl Iterator<Item = &mut SectionState> {
        self.roots
            .iter_mut()
            .flat_map(|root| root.sections.iter_mut())
    }
}

fn empty_section_state() -> SectionState {
    SectionState {
        id: "general".to_string(),
        title: "General".to_string(),
        description: None,
        presentation: Default::default(),
        required: true,
        default: None,
        path: Vec::new(),
        depth: 0,
        fields: Vec::new(),
        scroll_offset: 0,
    }
}

fn empty_root_state() -> RootSectionState {
    RootSectionState {
        id: "general".to_string(),
        title: "General".to_string(),
        description: None,
        sections: vec![empty_section_state()],
    }
}
