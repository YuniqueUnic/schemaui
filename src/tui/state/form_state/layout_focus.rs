use crate::tui::state::layout_nav::{NavRoot, NavSection};

use super::FormState;

#[derive(Clone)]
enum LayoutFocus {
    Field(String),
    Section { root: usize, section: usize },
}

impl FormState {
    pub fn focus_first_field_with_layout(&mut self) -> bool {
        let target = self.layout_nav.as_ref().and_then(|nav| {
            nav.roots.iter().find_map(|root| {
                root.sections
                    .iter()
                    .find_map(|section| self.layout_section_target(root, section))
            })
        });
        target.is_some_and(|target| self.focus_layout_target(target))
    }

    pub(super) fn focus_next_section_with_layout(&mut self, delta: i32) -> bool {
        let Some(nav) = &self.layout_nav else {
            return false;
        };
        let sections: Vec<_> = nav
            .roots
            .iter()
            .flat_map(|root| root.sections.iter().map(move |section| (root, section)))
            .filter_map(|(root, section)| {
                self.layout_section_target(root, section)
                    .map(|target| (root, section, target))
            })
            .collect();
        let Some(index) = sections
            .iter()
            .rposition(|(root, section, _)| self.layout_section_is_focused(root, section))
        else {
            return false;
        };
        let next = (index as i64 + i64::from(delta)).rem_euclid(sections.len() as i64) as usize;
        let target = sections[next].2.clone();
        self.focus_layout_target(target)
    }

    pub(super) fn focus_next_root_with_layout(&mut self, delta: i32) -> bool {
        let Some(nav) = &self.layout_nav else {
            return false;
        };
        let roots: Vec<_> = nav
            .roots
            .iter()
            .filter_map(|root| {
                root.sections
                    .iter()
                    .find_map(|section| self.layout_section_target(root, section))
                    .map(|target| (root, target))
            })
            .collect();
        let Some(index) = roots.iter().position(|(root, _)| {
            root.sections
                .iter()
                .any(|section| self.layout_section_is_focused(root, section))
        }) else {
            return false;
        };
        let next = (index as i64 + i64::from(delta)).rem_euclid(roots.len() as i64) as usize;
        let target = roots[next].1.clone();
        self.focus_layout_target(target)
    }

    fn layout_section_target(&self, root: &NavRoot, section: &NavSection) -> Option<LayoutFocus> {
        if let Some((root_index, section_index)) = self.layout_section_position(root, section) {
            let state = &self.roots[root_index].sections[section_index];
            if self.section_is_visible(state)
                && state.presentation.content.is_some()
                && !state
                    .fields
                    .iter()
                    .any(|field| self.field_is_visible(field))
            {
                return Some(LayoutFocus::Section {
                    root: root_index,
                    section: section_index,
                });
            }
        }
        section
            .first_pointer
            .iter()
            .chain(&section.pointers)
            .find(|pointer| {
                self.field_by_pointer(pointer)
                    .is_some_and(|field| self.field_is_visible(field))
            })
            .cloned()
            .map(LayoutFocus::Field)
    }

    fn layout_section_position(
        &self,
        root: &NavRoot,
        section: &NavSection,
    ) -> Option<(usize, usize)> {
        let root_index = self.roots.iter().position(|state| state.id == root.id)?;
        let section_index = self.roots[root_index]
            .sections
            .iter()
            .position(|state| state.id == section.id)?;
        Some((root_index, section_index))
    }

    fn layout_section_is_focused(&self, root: &NavRoot, section: &NavSection) -> bool {
        match self.focused_field() {
            Some(field) => section.pointers.contains(&field.schema.pointer),
            None => {
                self.layout_section_position(root, section)
                    == Some((self.root_index(), self.section_index()))
            }
        }
    }

    fn focus_layout_target(&mut self, target: LayoutFocus) -> bool {
        match target {
            LayoutFocus::Field(pointer) => self.focus_field_by_pointer(&pointer),
            LayoutFocus::Section { root, section } => {
                self.set_root_index(root);
                self.set_section_index(section);
                true
            }
        }
    }
}
