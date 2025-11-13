use serde_json::{Map, Value};

use crate::domain::FormSchema;

use super::{error::FieldCoercionError, field::FieldState, section::SectionState};

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
    pub root_index: usize,
    pub section_index: usize,
    pub field_index: usize,
}

impl FormState {
    pub fn from_schema(schema: &FormSchema) -> Self {
        let mut roots = Vec::new();
        if schema.roots.is_empty() {
            roots.push(empty_root_state());
        } else {
            for root in &schema.roots {
                let mut sections = Vec::new();
                for section in &root.sections {
                    SectionState::collect(section, 0, &mut sections);
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
            root_index: 0,
            section_index: 0,
            field_index: 0,
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
            root_index: 0,
            section_index: 0,
            field_index: 0,
        };
        state.normalize_focus();
        state
    }

    pub fn is_empty(&self) -> bool {
        self.roots.iter().all(|root| {
            root.sections
                .iter()
                .all(|section| section.fields.is_empty())
        })
    }

    pub fn active_root(&self) -> Option<&RootSectionState> {
        self.roots.get(self.root_index)
    }

    pub fn active_section(&self) -> Option<&SectionState> {
        self.active_root()
            .and_then(|root| root.sections.get(self.section_index))
    }

    pub fn active_section_mut(&mut self) -> Option<(&mut SectionState, usize)> {
        self.normalize_focus();
        let root = self.roots.get_mut(self.root_index)?;
        let section = root.sections.get_mut(self.section_index)?;
        let index = self.field_index.min(section.fields.len().saturating_sub(1));
        Some((section, index))
    }

    pub fn focused_field_mut(&mut self) -> Option<&mut FieldState> {
        let (section, index) = self.active_section_mut()?;
        section.fields.get_mut(index)
    }

    pub fn focused_field(&self) -> Option<&FieldState> {
        self.active_section()
            .and_then(|section| section.fields.get(self.field_index))
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
            self.root_index = root_idx;
            self.section_index = section_idx;
            self.field_index = field_idx;
            self.normalize_focus();
        }
    }

    pub fn focus_last_field(&mut self) {
        if let Some((root_idx, section_idx, field_idx)) = self.focus_positions().last().copied() {
            self.root_index = root_idx;
            self.section_index = section_idx;
            self.field_index = field_idx;
            self.normalize_focus();
        }
    }

    pub fn advance_focus_forward(&mut self) {
        self.normalize_focus();
        if self.roots.is_empty() {
            return;
        }
        if let Some(section) = self.active_section()
            && !section.fields.is_empty()
            && self.field_index + 1 < section.fields.len()
        {
            self.field_index += 1;
            return;
        }
        self.advance_section(1);
        self.field_index = 0;
        self.normalize_focus();
    }

    pub fn advance_focus_backward(&mut self) {
        self.normalize_focus();
        if self.roots.is_empty() {
            return;
        }
        if let Some(section) = self.active_section()
            && !section.fields.is_empty()
            && self.field_index > 0
        {
            self.field_index -= 1;
            return;
        }
        self.advance_section(-1);
        self.normalize_focus();
        if let Some(current) = self.active_section() {
            if current.fields.is_empty() {
                self.field_index = 0;
            } else {
                self.field_index = current.fields.len().saturating_sub(1);
            }
        }
    }

    pub fn focus_next_section(&mut self, delta: i32) {
        self.normalize_focus();
        self.advance_section(delta);
        self.normalize_focus();
    }

    pub fn focus_next_root(&mut self, delta: i32) {
        if self.roots.is_empty() {
            return;
        }
        let len = self.roots.len() as i32;
        let mut next = self.root_index as i32 + delta;
        if len > 0 {
            next = ((next % len) + len) % len;
        }
        self.root_index = next as usize;
        self.section_index = 0;
        self.field_index = 0;
        self.normalize_focus();
    }

    pub fn try_build_value(&self) -> Result<Value, FieldCoercionError> {
        let mut root = Value::Object(Map::new());
        for section in self.iter_sections() {
            for field in &section.fields {
                if let Some(value) = field.current_value()? {
                    insert_path(&mut root, &field.schema.path, value);
                }
            }
        }
        Ok(root)
    }

    pub fn seed_from_value(&mut self, value: &Value) {
        for section in self.iter_sections_mut() {
            for field in &mut section.fields {
                if let Some(subvalue) = value_at_path(value, &field.schema.path) {
                    field.seed_value(subvalue);
                }
            }
        }
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

    fn advance_section(&mut self, delta: i32) {
        let positions = self.section_positions();
        if positions.is_empty() {
            return;
        }
        let current_idx = positions
            .iter()
            .position(|&(root, section)| root == self.root_index && section == self.section_index)
            .unwrap_or(0);
        let len = positions.len() as i32;
        let mut next = current_idx as i32 + delta;
        next = ((next % len) + len) % len;
        let (root_index, section_index) = positions[next as usize];
        self.root_index = root_index;
        self.section_index = section_index;
        self.field_index = 0;
    }

    fn normalize_focus(&mut self) {
        if self.roots.is_empty() {
            return;
        }
        if self.root_index >= self.roots.len() {
            self.root_index = 0;
        }
        if self.roots[self.root_index].sections.is_empty() {
            if let Some((idx, _)) = self
                .roots
                .iter()
                .enumerate()
                .find(|(_, root)| !root.sections.is_empty())
            {
                self.root_index = idx;
            } else {
                self.section_index = 0;
                self.field_index = 0;
                return;
            }
        }
        let section_len = self.roots[self.root_index].sections.len();
        if section_len == 0 {
            self.section_index = 0;
            self.field_index = 0;
            return;
        }
        if self.section_index >= section_len {
            self.section_index = section_len - 1;
        }

        if !self.section_has_fields(self.root_index, self.section_index) {
            if let Some(index) = self.first_focusable_section_in_root(self.root_index) {
                self.section_index = index;
            } else if let Some((root_idx, section_idx)) =
                self.focusable_section_positions().first().copied()
            {
                self.root_index = root_idx;
                self.section_index = section_idx;
            }
        }

        let field_len = self.roots[self.root_index].sections[self.section_index]
            .fields
            .len();
        if field_len == 0 {
            self.field_index = 0;
        } else if self.field_index >= field_len {
            self.field_index = field_len - 1;
        }
    }

    fn iter_sections(&self) -> impl Iterator<Item = &SectionState> {
        self.roots.iter().flat_map(|root| root.sections.iter())
    }

    fn iter_sections_mut(&mut self) -> impl Iterator<Item = &mut SectionState> {
        self.roots
            .iter_mut()
            .flat_map(|root| root.sections.iter_mut())
    }

    fn current_focus_position(&self) -> Option<(usize, usize, usize)> {
        let root = self.roots.get(self.root_index)?;
        let section = root.sections.get(self.section_index)?;
        if section.fields.is_empty() {
            return None;
        }
        let field_len = section.fields.len();
        Some((
            self.root_index,
            self.section_index,
            self.field_index.min(field_len.saturating_sub(1)),
        ))
    }

    fn focus_positions(&self) -> Vec<(usize, usize, usize)> {
        let mut positions = Vec::new();
        for (root_idx, root) in self.roots.iter().enumerate() {
            for (section_idx, section) in root.sections.iter().enumerate() {
                for (field_idx, _field) in section.fields.iter().enumerate() {
                    positions.push((root_idx, section_idx, field_idx));
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
                if !section.fields.is_empty() {
                    positions.push((root_idx, section_idx));
                }
            }
        }
        positions
    }

    fn all_section_positions(&self) -> Vec<(usize, usize)> {
        let mut positions = Vec::new();
        for (root_idx, root) in self.roots.iter().enumerate() {
            for (section_idx, _section) in root.sections.iter().enumerate() {
                positions.push((root_idx, section_idx));
            }
        }
        positions
    }

    fn section_has_fields(&self, root_idx: usize, section_idx: usize) -> bool {
        self.roots
            .get(root_idx)
            .and_then(|root| root.sections.get(section_idx))
            .map(|section| !section.fields.is_empty())
            .unwrap_or(false)
    }

    fn first_focusable_section_in_root(&self, root_idx: usize) -> Option<usize> {
        self.roots.get(root_idx).and_then(|root| {
            root.sections
                .iter()
                .enumerate()
                .find(|(_, section)| !section.fields.is_empty())
                .map(|(idx, _)| idx)
        })
    }
}

fn insert_path(root: &mut Value, path: &[String], value: Value) {
    if path.is_empty() {
        *root = value;
        return;
    }

    if !root.is_object() {
        *root = Value::Object(Map::new());
    }

    if let Value::Object(obj) = root {
        if path.len() == 1 {
            obj.insert(path[0].clone(), value);
            return;
        }

        let entry = obj
            .entry(path[0].clone())
            .or_insert_with(|| Value::Object(Map::new()));
        insert_path(entry, &path[1..], value);
    }
}

fn empty_section_state() -> SectionState {
    SectionState {
        id: "general".to_string(),
        title: "General".to_string(),
        description: None,
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

fn value_at_path<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        match current {
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            _ => return None,
        }
    }
    Some(current)
}
