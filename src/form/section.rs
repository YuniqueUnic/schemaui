use crate::domain::FormSection;

use super::field::FieldState;

#[derive(Debug, Clone)]
pub struct SectionState {
    #[allow(dead_code)]
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    #[allow(dead_code)]
    pub path: Vec<String>,
    pub depth: usize,
    pub fields: Vec<FieldState>,
    pub scroll_offset: usize,
}

impl SectionState {
    pub fn collect(section: &FormSection, depth: usize, acc: &mut Vec<SectionState>) {
        let fields = section
            .fields
            .iter()
            .cloned()
            .map(FieldState::from_schema)
            .collect();
        acc.push(SectionState {
            id: section.id.clone(),
            title: section.title.clone(),
            description: section.description.clone(),
            path: section.path.clone(),
            depth,
            fields,
            scroll_offset: 0,
        });
        for child in &section.children {
            SectionState::collect(child, depth + 1, acc);
        }
    }
}
