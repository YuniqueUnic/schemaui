use super::{FieldState, FormState, RootSectionState, SectionState};

impl FormState {
    pub fn root_index(&self) -> usize {
        self.ui.root.current()
    }

    pub fn section_index(&self) -> usize {
        self.ui.sections.current()
    }

    pub fn field_index(&self) -> usize {
        self.ui.fields.current()
    }

    pub fn set_root_index(&mut self, index: usize) -> bool {
        let changed = self.ui.root.set(index, self.roots.len());
        if changed {
            self.ui.sections.reset();
            self.ui.fields.reset();
            self.normalize_focus();
        }
        changed
    }

    pub fn set_section_index(&mut self, index: usize) -> bool {
        let len = self
            .roots
            .get(self.root_index())
            .map(|root| root.sections.len())
            .unwrap_or(0);
        let changed = self.ui.sections.set(index, len);
        if changed {
            self.ui.fields.reset();
            self.normalize_focus();
        }
        changed
    }

    pub fn set_field_index(&mut self, index: usize) -> bool {
        let len = self
            .active_section()
            .map(|section| section.fields.len())
            .unwrap_or(0);
        let changed = self.ui.fields.set(index, len);
        if changed {
            self.normalize_focus();
        }
        changed
    }

    pub fn is_empty(&self) -> bool {
        self.roots.iter().all(|root| {
            root.sections
                .iter()
                .all(|section| section.fields.is_empty() && section.presentation.content.is_none())
        })
    }

    pub fn active_root(&self) -> Option<&RootSectionState> {
        self.roots.get(self.root_index())
    }

    pub fn active_section(&self) -> Option<&SectionState> {
        self.active_root()
            .and_then(|root| root.sections.get(self.section_index()))
            .filter(|section| self.section_is_visible(section))
    }

    pub fn active_section_mut(&mut self) -> Option<(&mut SectionState, usize)> {
        self.normalize_focus();
        let root_index = self.root_index();
        let section_index = self.section_index();
        let field_index = self.field_index();
        let root = self.roots.get_mut(root_index)?;
        let section = root.sections.get_mut(section_index)?;
        let index = field_index.min(section.fields.len().saturating_sub(1));
        Some((section, index))
    }

    pub fn focused_field_mut(&mut self) -> Option<&mut FieldState> {
        self.normalize_focus();
        self.focused_field()?;
        let (section, index) = self.active_section_mut()?;
        section.fields.get_mut(index)
    }

    pub fn focused_field(&self) -> Option<&FieldState> {
        self.active_section()
            .and_then(|section| section.fields.get(self.field_index()))
            .filter(|field| self.field_is_visible(field))
    }

    pub fn focus_next_field(&mut self) {
        self.advance_focus_forward();
    }

    pub fn focus_prev_field(&mut self) {
        self.advance_focus_backward();
    }

    pub fn has_focusable_fields(&self) -> bool {
        !self.focus_positions().is_empty()
    }

    pub fn focus_is_first(&self) -> bool {
        let positions = self.focus_positions();
        if positions.is_empty() {
            return false;
        }
        match (self.current_focus_position(), positions.first()) {
            (Some(current), Some(first)) => current == *first,
            _ => false,
        }
    }

    pub fn focus_is_last(&self) -> bool {
        let positions = self.focus_positions();
        if positions.is_empty() {
            return false;
        }
        match (self.current_focus_position(), positions.last()) {
            (Some(current), Some(last)) => current == *last,
            _ => false,
        }
    }

    pub fn focus_first_field(&mut self) {
        if let Some((root_idx, section_idx, field_idx)) = self.focus_positions().first().copied() {
            self.set_root_index(root_idx);
            self.set_section_index(section_idx);
            self.set_field_index(field_idx);
        }
    }

    pub fn focus_last_field(&mut self) {
        if let Some((root_idx, section_idx, field_idx)) = self.focus_positions().last().copied() {
            self.set_root_index(root_idx);
            self.set_section_index(section_idx);
            self.set_field_index(field_idx);
        }
    }

    fn advance_focus_by(&mut self, delta: i32) {
        self.normalize_focus();
        if self.roots.is_empty() {
            return;
        }
        let positions = self.focus_positions();
        if positions.is_empty() {
            return;
        }
        let current = self.current_focus_position();
        let next = match current.and_then(|curr| positions.iter().position(|pos| *pos == curr)) {
            Some(index) => {
                (index as i64 + i64::from(delta)).rem_euclid(positions.len() as i64) as usize
            }
            None if delta < 0 => positions
                .iter()
                .rposition(|&(root, section, _)| {
                    (root, section) < (self.root_index(), self.section_index())
                })
                .unwrap_or(positions.len() - 1),
            None => positions
                .iter()
                .position(|&(root, section, _)| {
                    (root, section) > (self.root_index(), self.section_index())
                })
                .unwrap_or(0),
        };
        let (root_idx, section_idx, field_idx) = positions[next];
        self.set_root_index(root_idx);
        self.set_section_index(section_idx);
        self.set_field_index(field_idx);
        self.normalize_focus();
    }

    pub fn advance_focus_forward(&mut self) {
        self.advance_focus_by(1);
    }

    pub fn advance_focus_backward(&mut self) {
        self.advance_focus_by(-1);
    }

    pub fn focus_next_section(&mut self, delta: i32) {
        if !self.focus_next_section_with_layout(delta) {
            self.normalize_focus();
            self.advance_section(delta);
            self.normalize_focus();
        }
    }

    pub fn focus_next_root(&mut self, delta: i32) {
        if !self.focus_next_root_with_layout(delta) {
            let visible = self.visible_root_indices();
            if visible.is_empty() {
                return;
            }
            let current = visible
                .iter()
                .position(|index| *index == self.root_index())
                .unwrap_or(0);
            let next =
                (current as i64 + i64::from(delta)).rem_euclid(visible.len() as i64) as usize;
            self.set_root_index(visible[next]);
            self.normalize_focus();
        }
    }

    pub(super) fn focus_field_by_pointer(&mut self, pointer: &str) -> bool {
        for (root_idx, root) in self.roots.iter().enumerate() {
            for (section_idx, section) in root.sections.iter().enumerate() {
                for (field_idx, field) in section.fields.iter().enumerate() {
                    if field.schema.pointer == pointer && self.field_is_visible(field) {
                        self.set_root_index(root_idx);
                        self.set_section_index(section_idx);
                        self.set_field_index(field_idx);
                        return true;
                    }
                }
            }
        }
        false
    }

    fn advance_section(&mut self, delta: i32) {
        let positions = self.section_positions();
        if positions.is_empty() {
            return;
        }
        let current_idx = positions
            .iter()
            .position(|&(root, section)| {
                root == self.root_index() && section == self.section_index()
            })
            .unwrap_or(0);
        let len = positions.len() as i32;
        let mut next = current_idx as i32 + delta;
        next = ((next % len) + len) % len;
        let (root_index, section_index) = positions[next as usize];
        self.set_root_index(root_index);
        self.set_section_index(section_index);
        self.ui.fields.reset();
    }

    pub(crate) fn normalize_focus(&mut self) {
        if self.roots.is_empty() {
            return;
        }
        self.ui.root.clamp(self.roots.len());
        if self
            .roots
            .get(self.root_index())
            .map(|root| root.sections.is_empty())
            .unwrap_or(true)
        {
            if let Some((idx, _)) = self
                .roots
                .iter()
                .enumerate()
                .find(|(_, root)| !root.sections.is_empty())
            {
                self.ui.root.set(idx, self.roots.len());
            } else {
                self.ui.sections.reset();
                self.ui.fields.reset();
                return;
            }
        }
        let section_len = self
            .roots
            .get(self.root_index())
            .map(|root| root.sections.len())
            .unwrap_or(0);
        if section_len == 0 {
            self.ui.sections.reset();
            self.ui.fields.reset();
            return;
        }
        self.ui.sections.clamp(section_len);

        if !self.section_has_fields(self.root_index(), self.section_index()) {
            if let Some(index) = self.first_focusable_section_in_root(self.root_index()) {
                self.ui.sections.set(index, section_len);
            } else if let Some((root_idx, section_idx)) =
                self.focusable_section_positions().first().copied()
            {
                self.ui.root.set(root_idx, self.roots.len());
                let new_len = self
                    .roots
                    .get(root_idx)
                    .map(|root| root.sections.len())
                    .unwrap_or(0);
                self.ui.sections.set(section_idx, new_len);
            }
        }

        let field_len = self
            .active_section()
            .map(|section| section.fields.len())
            .unwrap_or(0);
        self.ui.fields.clamp(field_len);
        let visible = self.visible_field_indices();
        if !visible.contains(&self.field_index())
            && let Some(index) = visible.first()
        {
            self.ui.fields.set(*index, field_len);
        }
    }

    fn current_focus_position(&self) -> Option<(usize, usize, usize)> {
        let root_idx = self.root_index();
        let section_idx = self.section_index();
        let root = self.roots.get(root_idx)?;
        let section = root.sections.get(section_idx)?;
        if section.fields.is_empty() {
            return None;
        }
        let field_len = section.fields.len();
        Some((
            root_idx,
            section_idx,
            self.field_index().min(field_len.saturating_sub(1)),
        ))
    }

    fn focus_positions(&self) -> Vec<(usize, usize, usize)> {
        let mut positions = Vec::new();
        for (root_idx, root) in self.roots.iter().enumerate() {
            for (section_idx, section) in root.sections.iter().enumerate() {
                for (field_idx, field) in section.fields.iter().enumerate() {
                    if self.field_is_visible(field) {
                        positions.push((root_idx, section_idx, field_idx));
                    }
                }
            }
        }
        positions
    }

    fn section_positions(&self) -> Vec<(usize, usize)> {
        let focusable = self.focusable_section_positions();
        if focusable.is_empty() {
            self.all_section_positions()
        } else {
            focusable
        }
    }

    fn focusable_section_positions(&self) -> Vec<(usize, usize)> {
        let mut positions = Vec::new();
        for (root_idx, root) in self.roots.iter().enumerate() {
            for (section_idx, section) in root.sections.iter().enumerate() {
                if self.section_is_visible(section)
                    && (section.presentation.content.is_some()
                        || section
                            .fields
                            .iter()
                            .any(|field| self.field_is_visible(field)))
                {
                    positions.push((root_idx, section_idx));
                }
            }
        }
        positions
    }

    fn all_section_positions(&self) -> Vec<(usize, usize)> {
        let mut positions = Vec::new();
        for (root_idx, root) in self.roots.iter().enumerate() {
            for (section_idx, section) in root.sections.iter().enumerate() {
                if self.section_is_visible(section) {
                    positions.push((root_idx, section_idx));
                }
            }
        }
        positions
    }

    fn section_has_fields(&self, root_idx: usize, section_idx: usize) -> bool {
        self.roots
            .get(root_idx)
            .and_then(|root| root.sections.get(section_idx))
            .map(|section| {
                self.section_is_visible(section)
                    && (section.presentation.content.is_some()
                        || section
                            .fields
                            .iter()
                            .any(|field| self.field_is_visible(field)))
            })
            .unwrap_or(false)
    }

    fn first_focusable_section_in_root(&self, root_idx: usize) -> Option<usize> {
        self.roots.get(root_idx).and_then(|root| {
            root.sections
                .iter()
                .enumerate()
                .find(|(_, section)| {
                    self.section_is_visible(section)
                        && (section.presentation.content.is_some()
                            || section
                                .fields
                                .iter()
                                .any(|field| self.field_is_visible(field)))
                })
                .map(|(idx, _)| idx)
        })
    }
}
