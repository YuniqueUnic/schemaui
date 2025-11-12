use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;

use crate::domain::{CompositeField, FieldKind, FieldSchema, KeyValueField};
use crate::form::{
    array::{ArrayEditorContext, ArrayEditorSession, ScalarArrayState},
    composite::{
        CompositeEditorSession, CompositeListEditorContext, CompositeListState, CompositeState,
        CompositeVariantSummary,
    },
    error::FieldCoercionError,
    key_value::{KeyValueEditorContext, KeyValueEditorSession, KeyValueState},
};

use super::convert::{
    adjust_numeric_value, array_to_string, array_value, default_text, integer_value, number_value,
    string_value, value_to_string,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    TextInput,
    ArrayBuffer,
    Bool,
    Enum,
    MultiSelect,
    Composite,
    CompositeList,
    ScalarArray,
    KeyValue,
}

pub trait FieldComponent: FieldComponentClone + std::fmt::Debug {
    fn kind(&self) -> ComponentKind;
    fn display_value(&self, schema: &FieldSchema) -> String;
    fn handle_key(&mut self, schema: &FieldSchema, key: &KeyEvent) -> bool {
        let _ = (schema, key);
        false
    }
    fn seed_value(&mut self, schema: &FieldSchema, value: &Value);
    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError>;

    fn bool_value(&self) -> Option<bool> {
        None
    }

    fn set_bool(&mut self, _value: bool) -> bool {
        false
    }

    fn enum_state(&self) -> Option<EnumStateRef<'_>> {
        None
    }

    fn set_enum_index(&mut self, _index: usize) -> bool {
        false
    }

    fn multi_state(&self) -> Option<MultiSelectStateRef<'_>> {
        None
    }

    fn set_multi_state(&mut self, _flags: &[bool]) -> bool {
        false
    }

    fn composite_popup(&self) -> Option<CompositePopupData> {
        None
    }

    fn composite_selector(&self) -> Option<CompositeSelectorView> {
        None
    }

    fn composite_summaries(&self) -> Option<Vec<CompositeVariantSummary>> {
        None
    }

    fn active_composite_variants(&self) -> Vec<usize> {
        Vec::new()
    }

    fn apply_composite_selection(&mut self, _selection: usize, _flags: Option<Vec<bool>>) -> bool {
        false
    }

    fn open_composite_editor(
        &mut self,
        pointer: &str,
        variant_index: usize,
    ) -> Result<CompositeEditorSession, FieldCoercionError> {
        let _ = variant_index;
        Err(FieldCoercionError::unsupported(
            pointer,
            "composite editing",
        ))
    }

    fn restore_composite_editor(&mut self, _session: CompositeEditorSession) {}

    fn collection_panel(&self) -> Option<(Vec<String>, usize)> {
        None
    }

    fn collection_selected_label(&self) -> Option<String> {
        None
    }

    fn collection_selected_index(&self) -> Option<usize> {
        None
    }

    fn collection_select(&mut self, _delta: i32) -> bool {
        false
    }

    fn collection_add(&mut self) -> bool {
        false
    }

    fn collection_remove(&mut self) -> bool {
        false
    }

    fn collection_move(&mut self, _delta: i32) -> bool {
        false
    }

    fn open_composite_list_editor(
        &mut self,
        pointer: &str,
    ) -> Result<CompositeListEditorContext, FieldCoercionError> {
        Err(FieldCoercionError::unsupported(
            pointer,
            "composite list editing",
        ))
    }

    fn restore_composite_list_editor(
        &mut self,
        _entry_index: usize,
        _session: CompositeEditorSession,
    ) {
    }

    fn open_key_value_editor(
        &mut self,
        pointer: &str,
    ) -> Result<KeyValueEditorContext, FieldCoercionError> {
        Err(FieldCoercionError::unsupported(pointer, "map editing"))
    }

    fn apply_key_value_editor(
        &mut self,
        _entry_index: usize,
        _session: &KeyValueEditorSession,
    ) -> Result<bool, FieldCoercionError> {
        Err(FieldCoercionError::unsupported("", "map editing"))
    }

    fn open_scalar_array_editor(
        &mut self,
        pointer: &str,
    ) -> Result<ArrayEditorContext, FieldCoercionError> {
        Err(FieldCoercionError::unsupported(pointer, "array editing"))
    }

    fn apply_scalar_array_editor(
        &mut self,
        _entry_index: usize,
        _session: &ArrayEditorSession,
    ) -> Result<bool, FieldCoercionError> {
        Err(FieldCoercionError::unsupported("", "array editing"))
    }
}

pub trait FieldComponentClone {
    fn clone_box(&self) -> Box<dyn FieldComponent>;
}

impl<T> FieldComponentClone for T
where
    T: 'static + FieldComponent + Clone,
{
    fn clone_box(&self) -> Box<dyn FieldComponent> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn FieldComponent> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone)]
pub struct EnumStateRef<'a> {
    pub options: &'a [String],
    pub selected: usize,
}

#[derive(Debug, Clone)]
pub struct MultiSelectStateRef<'a> {
    pub options: &'a [String],
    pub selected: &'a [bool],
}

#[derive(Debug, Clone)]
pub struct CompositeSelectorView {
    pub multi: bool,
    pub options: Vec<String>,
    pub active: Vec<bool>,
}

#[derive(Debug, Clone)]
pub struct CompositePopupData {
    pub options: Vec<String>,
    pub selected: usize,
    pub multi: bool,
    pub active: Vec<bool>,
}

#[derive(Debug, Clone)]
pub struct TextComponent {
    buffer: String,
}

impl TextComponent {
    pub fn new(schema: &FieldSchema) -> Self {
        Self {
            buffer: default_text(schema),
        }
    }
}

impl FieldComponent for TextComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::TextInput
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        self.buffer.clone()
    }

    fn handle_key(&mut self, schema: &FieldSchema, key: &KeyEvent) -> bool {
        handle_text_edit(&mut self.buffer, schema, key)
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        match value {
            Value::String(text) => self.buffer = text.clone(),
            Value::Number(num) => self.buffer = num.to_string(),
            other => self.buffer = value_to_string(other),
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        match schema.kind {
            FieldKind::String => string_value(&self.buffer, schema),
            FieldKind::Integer => integer_value(&self.buffer, schema),
            FieldKind::Number => number_value(&self.buffer, schema),
            FieldKind::Json => string_value(&self.buffer, schema),
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArrayBufferComponent {
    buffer: String,
}

impl ArrayBufferComponent {
    pub fn new(schema: &FieldSchema) -> Self {
        let buffer = schema
            .default
            .as_ref()
            .and_then(|value| value.as_array())
            .map(|items| array_to_string(items))
            .unwrap_or_default();
        Self { buffer }
    }
}

impl FieldComponent for ArrayBufferComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::ArrayBuffer
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        format!("[{}]", self.buffer.trim())
    }

    fn handle_key(&mut self, schema: &FieldSchema, key: &KeyEvent) -> bool {
        handle_text_edit(&mut self.buffer, schema, key)
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Value::Array(items) = value {
            self.buffer = array_to_string(items);
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        if let FieldKind::Array(inner) = &schema.kind {
            array_value(&self.buffer, inner.as_ref(), schema)
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoolComponent {
    value: bool,
}

impl BoolComponent {
    pub fn new(schema: &FieldSchema) -> Self {
        let value = schema
            .default
            .as_ref()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        Self { value }
    }
}

impl FieldComponent for BoolComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::Bool
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        self.value.to_string()
    }

    fn handle_key(&mut self, _schema: &FieldSchema, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right => {
                self.value = !self.value;
                true
            }
            _ => false,
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Some(flag) = value.as_bool() {
            self.value = flag;
        }
    }

    fn current_value(&self, _schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        Ok(Some(Value::Bool(self.value)))
    }

    fn bool_value(&self) -> Option<bool> {
        Some(self.value)
    }

    fn set_bool(&mut self, value: bool) -> bool {
        if self.value != value {
            self.value = value;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumComponent {
    options: Vec<String>,
    selected: usize,
}

impl EnumComponent {
    pub fn new(options: &[String], schema: &FieldSchema) -> Self {
        let default_value = schema
            .default
            .as_ref()
            .map(value_to_string)
            .and_then(|value| if value.is_empty() { None } else { Some(value) })
            .unwrap_or_else(|| options.first().cloned().unwrap_or_default());
        let selected = options
            .iter()
            .position(|item| item == &default_value)
            .unwrap_or(0);
        Self {
            options: options.to_vec(),
            selected,
        }
    }
}

impl FieldComponent for EnumComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::Enum
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        self.options
            .get(self.selected)
            .cloned()
            .unwrap_or_else(|| "<none>".to_string())
    }

    fn handle_key(&mut self, _schema: &FieldSchema, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Left => {
                if self.options.is_empty() {
                    return false;
                }
                if self.selected == 0 {
                    self.selected = self.options.len().saturating_sub(1);
                } else {
                    self.selected -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Right => {
                if self.options.is_empty() {
                    return false;
                }
                self.selected = (self.selected + 1) % self.options.len();
                true
            }
            _ => false,
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Some(text) = value.as_str() {
            if let Some(idx) = self.options.iter().position(|opt| opt == text) {
                self.selected = idx;
            }
        }
    }

    fn current_value(&self, _schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        Ok(self.options.get(self.selected).cloned().map(Value::String))
    }

    fn enum_state(&self) -> Option<EnumStateRef<'_>> {
        Some(EnumStateRef {
            options: &self.options,
            selected: self.selected,
        })
    }

    fn set_enum_index(&mut self, index: usize) -> bool {
        if self.options.is_empty() {
            return false;
        }
        let bounded = index.min(self.options.len() - 1);
        if self.selected != bounded {
            self.selected = bounded;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultiSelectComponent {
    options: Vec<String>,
    selected: Vec<bool>,
}

impl MultiSelectComponent {
    pub fn new(options: &[String], default: Option<&Value>) -> Self {
        let mut selected = vec![false; options.len()];
        if let Some(Value::Array(items)) = default {
            for item in items.iter().filter_map(Value::as_str) {
                if let Some(idx) = options.iter().position(|opt| opt == item) {
                    selected[idx] = true;
                }
            }
        }
        Self {
            options: options.to_vec(),
            selected,
        }
    }
}

impl FieldComponent for MultiSelectComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::MultiSelect
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        let values = self
            .options
            .iter()
            .zip(self.selected.iter())
            .filter_map(|(option, flag)| if *flag { Some(option.clone()) } else { None })
            .collect::<Vec<_>>();
        if values.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", values.join(", "))
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Value::Array(items) = value {
            let mut flags = vec![false; self.options.len()];
            for item in items.iter().filter_map(Value::as_str) {
                if let Some(idx) = self.options.iter().position(|opt| opt == item) {
                    flags[idx] = true;
                }
            }
            if flags.len() == self.selected.len() {
                self.selected = flags;
            }
        }
    }

    fn current_value(&self, _schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        let values = self
            .options
            .iter()
            .zip(self.selected.iter())
            .filter_map(|(option, flag)| {
                if *flag {
                    Some(Value::String(option.clone()))
                } else {
                    None
                }
            })
            .collect();
        Ok(Some(Value::Array(values)))
    }

    fn multi_state(&self) -> Option<MultiSelectStateRef<'_>> {
        Some(MultiSelectStateRef {
            options: &self.options,
            selected: &self.selected,
        })
    }

    fn set_multi_state(&mut self, flags: &[bool]) -> bool {
        if flags.len() != self.selected.len() {
            return false;
        }
        if self.selected != flags {
            self.selected.clone_from_slice(flags);
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompositeComponent {
    state: CompositeState,
}

impl CompositeComponent {
    pub fn new(pointer: &str, template: &CompositeField) -> Self {
        Self {
            state: CompositeState::new(pointer, template),
        }
    }
}

impl FieldComponent for CompositeComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::Composite
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        let mut label = self.state.summary();
        if self.state.is_multi() {
            label.push_str(" (Enter to toggle)");
        } else {
            label.push_str(" (Enter to choose)");
        }
        label
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if value.is_object() {
            let _ = self.state.seed_from_value(value);
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        self.state.build_value(schema.required)
    }

    fn composite_popup(&self) -> Option<CompositePopupData> {
        let options = self.state.option_titles();
        if options.is_empty() {
            return None;
        }
        let active = self.state.active_flags();
        let selected = self
            .state
            .selected_index()
            .unwrap_or(0)
            .min(options.len().saturating_sub(1));
        Some(CompositePopupData {
            options,
            selected,
            multi: self.state.is_multi(),
            active,
        })
    }

    fn composite_selector(&self) -> Option<CompositeSelectorView> {
        let options = self.state.option_titles();
        if options.is_empty() {
            return None;
        }
        Some(CompositeSelectorView {
            multi: self.state.is_multi(),
            options,
            active: self.state.active_flags(),
        })
    }

    fn composite_summaries(&self) -> Option<Vec<CompositeVariantSummary>> {
        let summaries = self.state.active_summaries();
        if summaries.is_empty() {
            None
        } else {
            Some(summaries)
        }
    }

    fn active_composite_variants(&self) -> Vec<usize> {
        self.state.active_indices()
    }

    fn apply_composite_selection(&mut self, selection: usize, flags: Option<Vec<bool>>) -> bool {
        if self.state.is_multi() {
            if let Some(flags) = flags {
                self.state.apply_multi(&flags)
            } else {
                false
            }
        } else {
            self.state.apply_single(selection)
        }
    }

    fn open_composite_editor(
        &mut self,
        pointer: &str,
        variant_index: usize,
    ) -> Result<CompositeEditorSession, FieldCoercionError> {
        self.state.take_editor_session(pointer, variant_index)
    }

    fn restore_composite_editor(&mut self, session: CompositeEditorSession) {
        self.state.restore_editor_session(session);
    }
}

#[derive(Debug, Clone)]
pub struct CompositeListComponent {
    state: CompositeListState,
}

impl CompositeListComponent {
    pub fn new(pointer: &str, template: &CompositeField, defaults: Option<&Value>) -> Self {
        Self {
            state: CompositeListState::new(pointer, template, defaults),
        }
    }
}

impl FieldComponent for CompositeListComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::CompositeList
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        let len = self.state.len();
        if len == 0 {
            "List: empty (Ctrl+N add)".to_string()
        } else {
            let selection = self
                .state
                .selected_label()
                .unwrap_or_else(|| "<no selection>".to_string());
            format!("List[{len}] • {selection} (Ctrl+Left/Right select, Ctrl+E edit)")
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Value::Array(items) = value {
            self.state.seed_entries_from_array(items);
        }
    }

    fn current_value(&self, _schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        self.state.build_value()
    }

    fn collection_panel(&self) -> Option<(Vec<String>, usize)> {
        self.state
            .selected_index()
            .map(|idx| (self.state.summaries(), idx))
    }

    fn collection_selected_label(&self) -> Option<String> {
        self.state.selected_label()
    }

    fn collection_selected_index(&self) -> Option<usize> {
        self.state.selected_index()
    }

    fn collection_select(&mut self, delta: i32) -> bool {
        self.state.select(delta)
    }

    fn collection_add(&mut self) -> bool {
        self.state.add_entry();
        true
    }

    fn collection_remove(&mut self) -> bool {
        self.state.remove_selected()
    }

    fn collection_move(&mut self, delta: i32) -> bool {
        self.state.move_selected(delta)
    }

    fn open_composite_list_editor(
        &mut self,
        pointer: &str,
    ) -> Result<CompositeListEditorContext, FieldCoercionError> {
        let _ = pointer;
        self.state.open_selected_editor()
    }

    fn restore_composite_list_editor(
        &mut self,
        entry_index: usize,
        session: CompositeEditorSession,
    ) {
        self.state.restore_entry_editor(entry_index, session);
    }
}

#[derive(Debug, Clone)]
pub struct ScalarArrayComponent {
    state: ScalarArrayState,
}

impl ScalarArrayComponent {
    pub fn new(schema: &FieldSchema, inner: &FieldKind) -> Self {
        Self {
            state: ScalarArrayState::new(
                &schema.pointer,
                schema.display_label(),
                schema.description.clone(),
                inner,
                schema.default.as_ref(),
            ),
        }
    }
}

impl FieldComponent for ScalarArrayComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::ScalarArray
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        let len = self.state.len();
        if len == 0 {
            "Array: empty (Ctrl+N add)".to_string()
        } else {
            let selection = self
                .state
                .selected_label()
                .unwrap_or_else(|| "<no selection>".to_string());
            format!("Array[{len}] • {selection} (Ctrl+Left/Right select, Ctrl+E edit)")
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Value::Array(items) = value {
            self.state.seed_entries_from_array(items);
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        self.state.build_value(schema.required)
    }

    fn collection_panel(&self) -> Option<(Vec<String>, usize)> {
        self.state.panel()
    }

    fn collection_selected_label(&self) -> Option<String> {
        self.state.selected_label()
    }

    fn collection_selected_index(&self) -> Option<usize> {
        self.state.selected_index()
    }

    fn collection_select(&mut self, delta: i32) -> bool {
        self.state.select(delta)
    }

    fn collection_add(&mut self) -> bool {
        self.state.add_entry()
    }

    fn collection_remove(&mut self) -> bool {
        self.state.remove_selected()
    }

    fn collection_move(&mut self, delta: i32) -> bool {
        self.state.move_selected(delta)
    }

    fn open_scalar_array_editor(
        &mut self,
        pointer: &str,
    ) -> Result<ArrayEditorContext, FieldCoercionError> {
        let _ = pointer;
        self.state.open_selected_editor()
    }

    fn apply_scalar_array_editor(
        &mut self,
        entry_index: usize,
        session: &ArrayEditorSession,
    ) -> Result<bool, FieldCoercionError> {
        self.state.apply_editor_session(entry_index, session)
    }
}

#[derive(Debug, Clone)]
pub struct KeyValueComponent {
    state: KeyValueState,
}

impl KeyValueComponent {
    pub fn new(pointer: &str, template: &KeyValueField, default: Option<&Value>) -> Self {
        Self {
            state: KeyValueState::new(pointer, template, default),
        }
    }
}

impl FieldComponent for KeyValueComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::KeyValue
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        let len = self.state.len();
        if len == 0 {
            "Map: empty (Ctrl+N add)".to_string()
        } else {
            let selection = self
                .state
                .selected_label()
                .unwrap_or_else(|| "<no selection>".to_string());
            format!("Map[{len}] • {selection} (Ctrl+Left/Right select, Ctrl+E edit)")
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Value::Object(map) = value {
            self.state.seed_entries_from_object(map);
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        self.state.build_value(schema.required)
    }

    fn collection_panel(&self) -> Option<(Vec<String>, usize)> {
        self.state.panel()
    }

    fn collection_selected_label(&self) -> Option<String> {
        self.state.selected_label()
    }

    fn collection_selected_index(&self) -> Option<usize> {
        self.state.selected_index()
    }

    fn collection_select(&mut self, delta: i32) -> bool {
        self.state.select(delta)
    }

    fn collection_add(&mut self) -> bool {
        self.state.add_entry()
    }

    fn collection_remove(&mut self) -> bool {
        self.state.remove_selected()
    }

    fn collection_move(&mut self, delta: i32) -> bool {
        self.state.move_selected(delta)
    }

    fn open_key_value_editor(
        &mut self,
        pointer: &str,
    ) -> Result<KeyValueEditorContext, FieldCoercionError> {
        let _ = pointer;
        self.state.open_selected_editor()
    }

    fn apply_key_value_editor(
        &mut self,
        entry_index: usize,
        session: &KeyValueEditorSession,
    ) -> Result<bool, FieldCoercionError> {
        self.state.apply_editor_session(entry_index, session)
    }
}

fn handle_text_edit(buffer: &mut String, schema: &FieldSchema, key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Left => adjust_numeric_value(buffer, &schema.kind, -1),
        KeyCode::Right => adjust_numeric_value(buffer, &schema.kind, 1),
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return false;
            }
            buffer.push(ch);
            true
        }
        KeyCode::Backspace => {
            buffer.pop();
            true
        }
        KeyCode::Delete => {
            buffer.clear();
            true
        }
        _ => false,
    }
}
