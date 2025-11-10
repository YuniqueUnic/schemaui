use crate::domain::FormSection;

use super::field::FieldState;

#[derive(Debug, Clone)]
pub struct SectionState {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub fields: Vec<FieldState>,
    pub scroll_offset: usize,
}

impl SectionState {
    pub fn from(section: &FormSection) -> Self {
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
            scroll_offset: 0,
        }
    }
}
