use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;

use crate::tui::model::{FieldKind, FieldSchema};
use crate::tui::state::error::FieldCoercionError;
use crate::tui::state::field::convert::{
    NumericStepValue, adjust_numeric_value, integer_value, nullable_value, number_value,
    string_value, value_to_string,
};

use super::{ComponentKind, FieldComponent, palette::ComponentPalette};

const MAX_HISTORY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextSnapshot {
    buffer: String,
    cursor: usize,
}

#[derive(Debug, Clone)]
pub struct TextComponent {
    buffer: String,
    cursor: usize,
    undo_stack: Vec<TextSnapshot>,
    redo_stack: Vec<TextSnapshot>,
    palette: Arc<ComponentPalette>,
}

impl TextComponent {
    pub fn new(schema: &FieldSchema, palette: Arc<ComponentPalette>) -> Self {
        let buffer = schema
            .default
            .as_ref()
            .map(value_to_string)
            .unwrap_or_default();
        let cursor = buffer.chars().count();
        Self {
            buffer,
            cursor,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            palette,
        }
    }

    fn snapshot(&self) -> TextSnapshot {
        TextSnapshot {
            buffer: self.buffer.clone(),
            cursor: self.cursor,
        }
    }

    fn restore_snapshot(&mut self, snapshot: TextSnapshot) {
        self.buffer = snapshot.buffer;
        self.cursor = snapshot.cursor.min(self.buffer.chars().count());
    }

    fn push_undo(&mut self) {
        if self.undo_stack.len() == MAX_HISTORY {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.snapshot());
    }

    fn commit_edit(&mut self, next_buffer: String, next_cursor: usize) -> bool {
        let next_cursor = next_cursor.min(next_buffer.chars().count());
        if self.buffer == next_buffer && self.cursor == next_cursor {
            return false;
        }
        self.push_undo();
        self.buffer = next_buffer;
        self.cursor = next_cursor;
        self.redo_stack.clear();
        true
    }

    fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.snapshot());
        self.restore_snapshot(previous);
        true
    }

    fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.push_undo();
        self.restore_snapshot(next);
        true
    }

    fn set_buffer(&mut self, next_buffer: String) {
        self.buffer = next_buffer;
        self.cursor = self.buffer.chars().count();
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    fn insert_char(&mut self, ch: char) -> bool {
        let mut next = self.buffer.clone();
        let byte_index = byte_index_for_char_offset(&next, self.cursor);
        next.insert(byte_index, ch);
        self.commit_edit(next, self.cursor + 1)
    }

    fn insert_numeric_char(&mut self, kind: &FieldKind, ch: char) -> bool {
        let mut next = self.buffer.clone();
        let byte_index = byte_index_for_char_offset(&next, self.cursor);
        next.insert(byte_index, ch);
        if !is_valid_numeric_edit_buffer(kind, &next) {
            return false;
        }
        self.commit_edit(next, self.cursor + 1)
    }

    fn move_cursor_left(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    fn move_cursor_right(&mut self) -> bool {
        let len = self.buffer.chars().count();
        if self.cursor >= len {
            return false;
        }
        self.cursor += 1;
        true
    }

    fn move_cursor_start(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor = 0;
        true
    }

    fn move_cursor_end(&mut self) -> bool {
        let len = self.buffer.chars().count();
        if self.cursor == len {
            return false;
        }
        self.cursor = len;
        true
    }

    fn move_line_boundary(&mut self, end: bool) -> bool {
        let chars: Vec<char> = self.buffer.chars().collect();
        let start = chars[..self.cursor]
            .iter()
            .rposition(|ch| *ch == '\n')
            .map_or(0, |i| i + 1);
        let next = if end {
            self.cursor
                + chars[self.cursor..]
                    .iter()
                    .position(|ch| *ch == '\n')
                    .unwrap_or(chars.len() - self.cursor)
        } else {
            start
        };
        let changed = next != self.cursor;
        self.cursor = next;
        changed
    }

    fn move_cursor_vertical(&mut self, delta: i32) -> bool {
        let chars: Vec<char> = self.buffer.chars().collect();
        let start = chars[..self.cursor]
            .iter()
            .rposition(|ch| *ch == '\n')
            .map_or(0, |i| i + 1);
        let column = self.cursor - start;
        let next = if delta < 0 {
            if start == 0 {
                return false;
            }
            let end = start - 1;
            let previous = chars[..end]
                .iter()
                .rposition(|ch| *ch == '\n')
                .map_or(0, |i| i + 1);
            previous + column.min(end - previous)
        } else {
            let Some(offset) = chars[self.cursor..].iter().position(|ch| *ch == '\n') else {
                return false;
            };
            let next = self.cursor + offset + 1;
            let length = chars[next..]
                .iter()
                .position(|ch| *ch == '\n')
                .unwrap_or(chars.len() - next);
            next + column.min(length)
        };
        self.cursor = next;
        true
    }

    fn delete_backward(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let mut next = self.buffer.clone();
        let end = byte_index_for_char_offset(&next, self.cursor);
        let start = byte_index_for_char_offset(&next, self.cursor - 1);
        next.replace_range(start..end, "");
        self.commit_edit(next, self.cursor - 1)
    }

    fn delete_forward(&mut self) -> bool {
        let len = self.buffer.chars().count();
        if self.cursor >= len {
            return false;
        }
        let mut next = self.buffer.clone();
        let start = byte_index_for_char_offset(&next, self.cursor);
        let end = byte_index_for_char_offset(&next, self.cursor + 1);
        next.replace_range(start..end, "");
        self.commit_edit(next, self.cursor)
    }

    fn delete_previous_word(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let chars: Vec<char> = self.buffer.chars().collect();
        let mut start = self.cursor.min(chars.len());
        while start > 0 && chars[start - 1].is_whitespace() {
            start -= 1;
        }
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }
        if start == self.cursor {
            return false;
        }
        let next: String = chars[..start]
            .iter()
            .chain(chars[self.cursor..].iter())
            .collect();
        self.commit_edit(next, start)
    }

    fn step_numeric(&mut self, schema: &FieldSchema, key: &KeyEvent) -> bool {
        let fast = key.modifiers.contains(KeyModifiers::SHIFT);
        if schema.presentation.control == Some(crate::ui_ast::FieldControl::Slider)
            && let Some(scale) = crate::tui::model::numeric_scale::NumericScale::from_schema(schema)
        {
            let direction = if key.code == KeyCode::Left { -1 } else { 1 };
            let current = self.buffer.trim().parse().ok();
            let next = scale.adjust(current, direction, fast).to_string();
            let cursor = next.chars().count();
            return self.commit_edit(next, cursor);
        }
        let sign = if matches!(key.code, KeyCode::Left) {
            -1.0
        } else {
            1.0
        };
        let delta = match schema.kind.non_nullable() {
            FieldKind::Integer => {
                let step = self.palette.numeric.step_i64(fast) as f64 * sign;
                NumericStepValue::Integer(step.round() as i64)
            }
            FieldKind::Number => {
                let step = self.palette.numeric.step_f64(fast) * sign;
                NumericStepValue::Float(step)
            }
            _ => return false,
        };

        let mut next = self.buffer.clone();
        if !adjust_numeric_value(&mut next, schema.kind.non_nullable(), delta) {
            return false;
        }
        let next_cursor = next.chars().count();
        self.commit_edit(next, next_cursor)
    }

    fn handle_string_like_key(&mut self, schema: &FieldSchema, key: &KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'w') => self.delete_previous_word(),
                KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'y') => self.redo(),
                KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'z') => self.undo(),
                KeyCode::Home => self.move_cursor_start(),
                KeyCode::End => self.move_cursor_end(),
                _ => false,
            };
        }

        match key.code {
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Home if schema.presentation.is_multiline() => self.move_line_boundary(false),
            KeyCode::End if schema.presentation.is_multiline() => self.move_line_boundary(true),
            KeyCode::Home => self.move_cursor_start(),
            KeyCode::End => self.move_cursor_end(),
            KeyCode::Enter if schema.presentation.is_multiline() => self.insert_char('\n'),
            KeyCode::Up if schema.presentation.is_multiline() => self.move_cursor_vertical(-1),
            KeyCode::Down if schema.presentation.is_multiline() => self.move_cursor_vertical(1),
            KeyCode::Backspace => self.delete_backward(),
            KeyCode::Delete => self.delete_forward(),
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::ALT) => self.insert_char(ch),
            _ => false,
        }
    }

    fn handle_numeric_key(&mut self, schema: &FieldSchema, key: &KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'y') => self.redo(),
                KeyCode::Char(ch) if ch.eq_ignore_ascii_case(&'z') => self.undo(),
                _ => false,
            };
        }

        match key.code {
            KeyCode::Left | KeyCode::Right => self.step_numeric(schema, key),
            KeyCode::Backspace => self.delete_backward(),
            KeyCode::Delete => self.delete_forward(),
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::ALT) => {
                self.insert_numeric_char(schema.kind.non_nullable(), ch)
            }
            _ => false,
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
        match schema.kind.non_nullable() {
            FieldKind::String | FieldKind::Json => self.handle_string_like_key(schema, key),
            FieldKind::Integer | FieldKind::Number => self.handle_numeric_key(schema, key),
            _ => false,
        }
    }

    fn seed_value(&mut self, schema: &FieldSchema, value: &Value) {
        if matches!(schema.kind, FieldKind::Json) {
            self.set_buffer(value.to_string());
            return;
        }
        match value {
            Value::String(text) => self.set_buffer(text.clone()),
            Value::Number(num) => self.set_buffer(num.to_string()),
            other => self.set_buffer(value_to_string(other)),
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        match schema.kind {
            FieldKind::String => string_value(&self.buffer, schema),
            FieldKind::Integer => integer_value(&self.buffer, schema),
            FieldKind::Number => number_value(&self.buffer, schema),
            FieldKind::Json => {
                if self.buffer.trim().is_empty() {
                    return Ok(None);
                }
                serde_json::from_str(&self.buffer)
                    .map(Some)
                    .map_err(|error| FieldCoercionError {
                        pointer: schema.pointer.clone(),
                        message: format!("expected JSON: {error}"),
                    })
            }
            FieldKind::Nullable(ref inner) => nullable_value(&self.buffer, inner, schema),
            _ => Ok(None),
        }
    }

    fn edit_buffer(&self) -> Option<&str> {
        Some(&self.buffer)
    }

    fn paste(&mut self, schema: &FieldSchema, text: &str) -> bool {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let text: String = normalized
            .chars()
            .filter_map(|ch| match ch {
                '\n' if !schema.presentation.is_multiline() => Some(' '),
                '\n' | '\t' => Some(ch),
                ch if !ch.is_control() => Some(ch),
                _ => None,
            })
            .collect();
        if text.is_empty() {
            return false;
        }
        let mut next = self.buffer.clone();
        next.insert_str(byte_index_for_char_offset(&next, self.cursor), &text);
        if matches!(
            schema.kind.non_nullable(),
            FieldKind::Integer | FieldKind::Number
        ) && !is_valid_numeric_edit_buffer(schema.kind.non_nullable(), &next)
        {
            return false;
        }
        self.commit_edit(next, self.cursor + text.chars().count())
    }

    fn cursor_offset(&self, schema: &FieldSchema) -> Option<usize> {
        match schema.kind.non_nullable() {
            FieldKind::String | FieldKind::Json => Some(self.cursor),
            _ => None,
        }
    }
}

fn byte_index_for_char_offset(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .map(|(idx, _)| idx)
        .nth(char_offset)
        .unwrap_or(text.len())
}

fn is_valid_numeric_edit_buffer(kind: &FieldKind, buffer: &str) -> bool {
    if buffer.is_empty() {
        return true;
    }

    match kind {
        FieldKind::Integer => {
            buffer.parse::<i64>().is_ok() || format!("{buffer}0").parse::<i64>().is_ok()
        }
        FieldKind::Number => {
            buffer.parse::<f64>().is_ok() || format!("{buffer}0").parse::<f64>().is_ok()
        }
        _ => false,
    }
}
