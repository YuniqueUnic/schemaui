use std::{borrow::Cow, fmt, num::NonZeroU16, str::FromStr, sync::Arc, time::Duration};

use anyhow::Result;

use super::{
    input::KeyBindingMap,
    keymap::{self, KeymapStore},
};
use crate::tui::state::field::components::{
    BoolTogglePresentation, CollectionHints, ComponentPalette, CompositeHints, EnumBehaviour,
    NumericTuning,
};

/// Terminal graphics selection. Automatic mode never queries terminal input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ImageProtocol {
    #[default]
    Auto,
    Kitty,
    Iterm2,
    Sixel,
    Source,
}

impl FromStr for ImageProtocol {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "kitty" => Ok(Self::Kitty),
            "iterm2" => Ok(Self::Iterm2),
            "sixel" => Ok(Self::Sixel),
            "source" => Ok(Self::Source),
            _ => Err("expected auto, kitty, iterm2, sixel or source".into()),
        }
    }
}

impl fmt::Display for ImageProtocol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Auto => "auto",
            Self::Kitty => "kitty",
            Self::Iterm2 => "iterm2",
            Self::Sixel => "sixel",
            Self::Source => "source",
        })
    }
}

/// Pixel dimensions of a terminal cell, when supplied by the host or user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageCellSize {
    width: NonZeroU16,
    height: NonZeroU16,
}

impl ImageCellSize {
    pub fn new(width: u16, height: u16) -> Option<Self> {
        Some(Self {
            width: NonZeroU16::new(width)?,
            height: NonZeroU16::new(height)?,
        })
    }

    pub fn width(self) -> u16 {
        self.width.get()
    }

    pub fn height(self) -> u16 {
        self.height.get()
    }
}

impl FromStr for ImageCellSize {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let invalid =
            || "expected positive cell pixels as WIDTHxHEIGHT, for example 10x20".to_string();
        let (width, height) = value.split_once('x').ok_or_else(invalid)?;
        Self::new(
            width.parse().map_err(|_| invalid())?,
            height.parse().map_err(|_| invalid())?,
        )
        .ok_or_else(invalid)
    }
}

#[derive(Debug, Clone)]
pub struct UiOptions {
    pub tick_rate: Duration,
    pub auto_validate: bool,
    pub confirm_exit: bool,
    pub show_help: bool,
    pub image_protocol: ImageProtocol,
    pub image_cell_size: Option<ImageCellSize>,
    pub keymap: KeyBindingMap,
    pub(crate) keymap_store: Arc<KeymapStore>,
    pub(crate) component_palette: Arc<ComponentPalette>,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
            auto_validate: true,
            confirm_exit: true,
            show_help: true,
            image_protocol: ImageProtocol::Auto,
            image_cell_size: None,
            keymap: KeyBindingMap::default(),
            keymap_store: keymap::default_store(),
            component_palette: Arc::new(ComponentPalette::default()),
        }
    }
}

impl UiOptions {
    pub fn with_image_protocol(mut self, protocol: ImageProtocol) -> Self {
        self.image_protocol = protocol;
        self
    }

    pub fn with_image_cell_size(mut self, cell_size: ImageCellSize) -> Self {
        self.image_cell_size = Some(cell_size);
        self
    }

    pub fn with_keymap(mut self, keymap: KeyBindingMap) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn with_keymap_json(self, raw: &str) -> Result<Self> {
        let keymap_store = Arc::new(KeymapStore::from_json(raw)?);
        Ok(self.with_keymap_store(keymap_store))
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

    pub fn with_component_palette(mut self, palette: ComponentPalette) -> Self {
        self.component_palette = Arc::new(palette);
        self
    }

    pub fn with_integer_step(self, step: i64) -> Self {
        self.map_palette(|mut palette| {
            palette.numeric = palette.numeric.with_integer_step(step);
            palette
        })
    }

    pub fn with_integer_fast_step(self, step: i64) -> Self {
        self.map_palette(|mut palette| {
            palette.numeric = palette.numeric.with_integer_fast_step(step);
            palette
        })
    }

    pub fn with_float_step(self, step: f64) -> Self {
        self.map_palette(|mut palette| {
            palette.numeric = palette.numeric.with_float_step(step);
            palette
        })
    }

    pub fn with_float_fast_step(self, step: f64) -> Self {
        self.map_palette(|mut palette| {
            palette.numeric = palette.numeric.with_float_fast_step(step);
            palette
        })
    }

    pub fn with_numeric_tuning(mut self, tuning: NumericTuning) -> Self {
        let mut palette = (*self.component_palette).clone();
        palette.numeric = tuning;
        self.component_palette = Arc::new(palette);
        self
    }

    pub fn with_bool_presentation(mut self, presentation: BoolTogglePresentation) -> Self {
        let mut palette = (*self.component_palette).clone();
        palette.bools = presentation;
        self.component_palette = Arc::new(palette);
        self
    }

    pub fn with_bool_labels(
        self,
        true_label: impl Into<Cow<'static, str>>,
        false_label: impl Into<Cow<'static, str>>,
    ) -> Self {
        let true_label = true_label.into();
        let false_label = false_label.into();
        self.map_palette(|mut palette| {
            palette.bools = palette.bools.clone().with_labels(true_label, false_label);
            palette
        })
    }

    pub fn with_bool_toggle_arrows(self, enabled: bool) -> Self {
        self.map_palette(|mut palette| {
            palette.bools = palette.bools.clone().with_toggle_with_arrows(enabled);
            palette
        })
    }

    pub fn with_bool_toggle_space(self, enabled: bool) -> Self {
        self.map_palette(|mut palette| {
            palette.bools = palette.bools.clone().with_toggle_with_space(enabled);
            palette
        })
    }

    pub fn with_enum_behaviour(mut self, behaviour: EnumBehaviour) -> Self {
        let mut palette = (*self.component_palette).clone();
        palette.enums = behaviour;
        self.component_palette = Arc::new(palette);
        self
    }

    pub fn with_enum_wrap(self, wrap: bool) -> Self {
        self.map_palette(|mut palette| {
            palette.enums = palette.enums.clone().with_wrap(wrap);
            palette
        })
    }

    pub fn with_collection_hints(mut self, hints: CollectionHints) -> Self {
        let mut palette = (*self.component_palette).clone();
        palette.collection = hints;
        self.component_palette = Arc::new(palette);
        self
    }

    pub fn with_overlay_instructions(self, instructions: impl Into<Cow<'static, str>>) -> Self {
        let instructions = instructions.into();
        self.map_palette(|mut palette| {
            palette.collection = palette
                .collection
                .clone()
                .with_overlay_instructions(instructions);
            palette
        })
    }

    pub fn with_list_hint(self, hint: impl Into<Cow<'static, str>>) -> Self {
        let hint = hint.into();
        self.map_palette(|mut palette| {
            palette.collection = palette.collection.clone().with_list_hint(hint);
            palette
        })
    }

    pub fn with_composite_hints(mut self, hints: CompositeHints) -> Self {
        let mut palette = (*self.component_palette).clone();
        palette.composite = hints;
        self.component_palette = Arc::new(palette);
        self
    }

    pub fn with_composite_single_hint(self, hint: impl Into<Cow<'static, str>>) -> Self {
        let hint = hint.into();
        self.map_palette(|mut palette| {
            palette.composite = palette.composite.clone().with_single_hint(hint);
            palette
        })
    }

    pub fn with_composite_multi_hint(self, hint: impl Into<Cow<'static, str>>) -> Self {
        let hint = hint.into();
        self.map_palette(|mut palette| {
            palette.composite = palette.composite.clone().with_multi_hint(hint);
            palette
        })
    }

    pub fn component_palette(&self) -> Arc<ComponentPalette> {
        Arc::clone(&self.component_palette)
    }

    fn map_palette(mut self, map: impl FnOnce(ComponentPalette) -> ComponentPalette) -> Self {
        let updated = map((*self.component_palette).clone());
        self.component_palette = Arc::new(updated);
        self
    }
}
