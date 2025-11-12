use std::sync::Arc;
use std::time::Duration;

use super::{
    input::KeyBindingMap,
    keymap::{self, KeymapStore},
};

#[derive(Debug, Clone)]
pub struct UiOptions {
    pub tick_rate: Duration,
    pub auto_validate: bool,
    pub confirm_exit: bool,
    pub show_help: bool,
    pub keymap: KeyBindingMap,
    pub(crate) keymap_store: Arc<KeymapStore>,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
            auto_validate: true,
            confirm_exit: true,
            show_help: true,
            keymap: KeyBindingMap::default(),
            keymap_store: keymap::default_store(),
        }
    }
}

impl UiOptions {
    pub fn with_keymap(mut self, keymap: KeyBindingMap) -> Self {
        self.keymap = keymap;
        self
    }

    pub(crate) fn with_keymap_store(mut self, keymap_store: Arc<KeymapStore>) -> Self {
        self.keymap_store = keymap_store;
        self
    }

    pub fn with_auto_validate(mut self, enabled: bool) -> Self {
        self.auto_validate = enabled;
        self
    }

    pub fn with_help(mut self, show: bool) -> Self {
        self.show_help = show;
        self
    }

    pub fn with_confirm_exit(mut self, confirm: bool) -> Self {
        self.confirm_exit = confirm;
        self
    }

    pub fn with_tick_rate(mut self, tick_rate: Duration) -> Self {
        self.tick_rate = tick_rate;
        self
    }
}
