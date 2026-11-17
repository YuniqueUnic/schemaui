use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use serde_json::Value;

use crate::tui::{
    model::{FieldKind, FieldSchema},
    state::FieldCoercionError,
};
use crate::ui_ast::FieldControl;

use super::{ComponentKind, ComponentPalette, FieldComponent, TextComponent};

#[derive(Debug, Clone)]
pub struct RangeComponent {
    endpoints: [TextComponent; 2],
    active: usize,
    scalar: FieldSchema,
}

impl RangeComponent {
    pub fn new(schema: &FieldSchema, palette: Arc<ComponentPalette>) -> Self {
        let mut scalar = schema.clone();
        if let FieldKind::Array(inner) = &schema.kind {
            scalar.kind = *inner.clone();
        }
        scalar.default = None;
        scalar.required = true;
        scalar.presentation.control = Some(FieldControl::Slider);
        Self {
            endpoints: [
                TextComponent::new(&scalar, palette.clone()),
                TextComponent::new(&scalar, palette),
            ],
            active: 0,
            scalar,
        }
    }
}

impl FieldComponent for RangeComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::Range
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        format!(
            "{}Lower: {}\n{}Upper: {}",
            if self.active == 0 { "> " } else { "  " },
            self.endpoints[0].display_value(&self.scalar),
            if self.active == 1 { "> " } else { "  " },
            self.endpoints[1].display_value(&self.scalar)
        )
    }

    fn handle_key(&mut self, _schema: &FieldSchema, key: &KeyEvent) -> bool {
        if matches!(key.code, KeyCode::Up | KeyCode::Down)
            && key.modifiers.intersects(
                crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::ALT,
            )
        {
            return false;
        }
        match key.code {
            KeyCode::Up => {
                self.active = 0;
                true
            }
            KeyCode::Down => {
                self.active = 1;
                true
            }
            _ => {
                let previous = self.endpoints[self.active].clone();
                let handled = self.endpoints[self.active].handle_key(&self.scalar, key);
                if matches!(key.code, KeyCode::Left | KeyCode::Right)
                    && let (Ok(Some(lower)), Ok(Some(upper))) = (
                        self.endpoints[0].current_value(&self.scalar),
                        self.endpoints[1].current_value(&self.scalar),
                    )
                    && lower
                        .as_f64()
                        .zip(upper.as_f64())
                        .is_some_and(|(a, b)| a > b)
                {
                    self.endpoints[self.active] = previous;
                    return false;
                }
                handled
            }
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Some(values) = value.as_array().filter(|values| values.len() == 2) {
            for (endpoint, value) in self.endpoints.iter_mut().zip(values) {
                endpoint.seed_value(&self.scalar, value);
            }
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        let lower = self.endpoints[0].current_value(&self.scalar)?;
        let upper = self.endpoints[1].current_value(&self.scalar)?;
        match (lower, upper) {
            (Some(lower), Some(upper))
                if lower
                    .as_f64()
                    .zip(upper.as_f64())
                    .is_some_and(|(a, b)| a <= b) =>
            {
                Ok(Some(Value::Array(vec![lower, upper])))
            }
            _ => Err(FieldCoercionError {
                pointer: schema.pointer.clone(),
                message: "range needs two numbers with lower <= upper".to_string(),
            }),
        }
    }

    fn edit_buffer(&self) -> Option<&str> {
        self.endpoints[self.active].edit_buffer()
    }

    fn range_endpoint(&self) -> Option<usize> {
        Some(self.active)
    }

    fn paste(&mut self, _schema: &FieldSchema, text: &str) -> bool {
        self.endpoints[self.active].paste(&self.scalar, text)
    }
}
