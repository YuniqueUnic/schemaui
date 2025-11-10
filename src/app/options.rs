use std::time::Duration;

use super::input::KeyBindingMap;

#[derive(Debug, Clone)]
pub struct UiOptions {
    pub tick_rate: Duration,
    pub auto_validate: bool,
    pub confirm_exit: bool,
    pub show_help: bool,
    pub keymap: KeyBindingMap,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
            auto_validate: true,
            confirm_exit: true,
            show_help: true,
            keymap: KeyBindingMap::default(),
        }
    }
}

impl UiOptions {
    pub fn with_keymap(mut self, keymap: KeyBindingMap) -> Self {
        self.keymap = keymap;
        self
    }
}
