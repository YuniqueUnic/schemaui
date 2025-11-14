use crate::form::{FormState, SectionState};

#[derive(Debug, Clone)]
pub struct RootTabsView {
    pub titles: Vec<String>,
    pub selected: usize,
}

#[derive(Debug, Clone)]
pub struct SectionTabsView {
    pub titles: Vec<String>,
    pub selected: usize,
    pub label: String,
}

pub struct FieldsView<'a> {
    pub section: &'a mut SectionState,
    pub selected: usize,
}

impl FormState {
    pub fn root_tabs_view(&self) -> Option<RootTabsView> {
        if self.roots.is_empty() {
            return None;
        }
        Some(RootTabsView {
            titles: self.roots.iter().map(|root| root.title.clone()).collect(),
            selected: self.root_index(),
        })
    }

    pub fn section_tabs_view(&self) -> Option<SectionTabsView> {
        let root = self.active_root()?;
        Some(SectionTabsView {
            titles: root
                .sections
                .iter()
                .map(|section| {
                    if section.depth == 0 {
                        section.title.clone()
                    } else {
                        format!("{}{}", "› ".repeat(section.depth), section.title)
                    }
                })
                .collect(),
            selected: self.section_index(),
            label: format!("{} Sections", root.title),
        })
    }

    pub fn fields_view(&mut self) -> Option<FieldsView<'_>> {
        self.active_section_mut()
            .map(|(section, selected)| FieldsView { section, selected })
    }
}
