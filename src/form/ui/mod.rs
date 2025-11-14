//! UI-focused stores that encapsulate navigational state for the form tree.
//! Each visual区域（root tabs、section tabs、field list）拥有自己的 store，
//! 以便 presentation 层像 React + Zustand 那样组合/订阅状态。

pub mod view;
pub use view::{FieldsView, RootTabsView, SectionTabsView};

#[derive(Debug, Clone, Copy, Default)]
pub struct RootTabsStore {
    current: usize,
}

impl RootTabsStore {
    pub fn current(&self) -> usize {
        self.current
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.current = 0;
    }

    pub fn clamp(&mut self, len: usize) {
        if len == 0 {
            self.current = 0;
        } else if self.current >= len {
            self.current = len.saturating_sub(1);
        }
    }

    pub fn set(&mut self, index: usize, len: usize) -> bool {
        let bounded = if len == 0 {
            0
        } else {
            index.min(len.saturating_sub(1))
        };
        if self.current == bounded {
            false
        } else {
            self.current = bounded;
            true
        }
    }

    pub fn advance(&mut self, delta: i32, len: usize) -> bool {
        if len == 0 {
            self.current = 0;
            return false;
        }
        let len_i32 = len as i32;
        let mut next = self.current as i32 + delta;
        next = ((next % len_i32) + len_i32) % len_i32;
        self.set(next as usize, len)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SectionTabsStore {
    current: usize,
}

impl SectionTabsStore {
    pub fn current(&self) -> usize {
        self.current
    }

    pub fn reset(&mut self) {
        self.current = 0;
    }

    pub fn clamp(&mut self, len: usize) {
        if len == 0 {
            self.current = 0;
        } else if self.current >= len {
            self.current = len.saturating_sub(1);
        }
    }

    pub fn set(&mut self, index: usize, len: usize) -> bool {
        let bounded = if len == 0 {
            0
        } else {
            index.min(len.saturating_sub(1))
        };
        if self.current == bounded {
            false
        } else {
            self.current = bounded;
            true
        }
    }

    #[allow(dead_code)]
    pub fn advance(&mut self, delta: i32, len: usize) -> bool {
        if len == 0 {
            self.current = 0;
            return false;
        }
        let len_i32 = len as i32;
        let mut next = self.current as i32 + delta;
        next = ((next % len_i32) + len_i32) % len_i32;
        self.set(next as usize, len)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FieldListStore {
    current: usize,
}

#[allow(dead_code)]
impl FieldListStore {
    pub fn current(&self) -> usize {
        self.current
    }

    pub fn reset(&mut self) {
        self.current = 0;
    }

    pub fn clamp(&mut self, len: usize) {
        if len == 0 {
            self.current = 0;
        } else if self.current >= len {
            self.current = len.saturating_sub(1);
        }
    }

    pub fn set(&mut self, index: usize, len: usize) -> bool {
        let bounded = if len == 0 {
            0
        } else {
            index.min(len.saturating_sub(1))
        };
        if self.current == bounded {
            false
        } else {
            self.current = bounded;
            true
        }
    }

    pub fn advance(&mut self, delta: i32, len: usize) -> bool {
        if len == 0 {
            self.current = 0;
            return false;
        }
        let len_i32 = len as i32;
        let mut next = self.current as i32 + delta;
        next = ((next % len_i32) + len_i32) % len_i32;
        self.set(next as usize, len)
    }
}

#[derive(Debug, Clone, Default)]
pub struct UiStores {
    pub root: RootTabsStore,
    pub sections: SectionTabsStore,
    pub fields: FieldListStore,
}

impl UiStores {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.root.reset();
        self.sections.reset();
        self.fields.reset();
    }
}
