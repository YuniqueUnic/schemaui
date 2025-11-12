use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;
use std::sync::LazyLock;

use super::input::KeyAction;

macro_rules! keymap_source {
    () => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/keymap/default.keymap.json"
        ))
    };
}

pub(super) use keymap_source;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum KeymapContext {
    Default,
    Collection,
    Overlay,
}

impl KeymapContext {
    fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "default" => Some(KeymapContext::Default),
            "collection" => Some(KeymapContext::Collection),
            "overlay" => Some(KeymapContext::Overlay),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
struct RawEntry {
    id: String,
    description: String,
    contexts: Vec<String>,
    action: RawAction,
    combos: Vec<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum RawAction {
    Save,
    Quit,
    ResetStatus,
    TogglePopup,
    EditComposite,
    FieldStep { delta: i32 },
    SectionStep { delta: i32 },
    RootStep { delta: i32 },
    ListAddEntry,
    ListRemoveEntry,
    ListMove { delta: i32 },
    ListSelect { delta: i32 },
}

struct KeyBinding {
    action: KeyAction,
    contexts: Vec<KeymapContext>,
    combos: Vec<KeyPattern>,
    snippet: String,
}

impl KeyBinding {
    fn from_raw(raw: RawEntry) -> Self {
        let contexts = raw
            .contexts
            .iter()
            .filter_map(|ctx| KeymapContext::from_str(ctx))
            .collect::<Vec<_>>();
        assert!(
            !contexts.is_empty(),
            "keymap entry {} must declare at least one context",
            raw.id
        );
        let action = raw.action.into_action();
        let combos = raw
            .combos
            .iter()
            .map(|combo| {
                KeyPattern::parse(combo).unwrap_or_else(|err| {
                    panic!("failed to parse combo '{combo}' for {}: {err}", raw.id)
                })
            })
            .collect::<Vec<_>>();
        assert!(
            !combos.is_empty(),
            "keymap entry {} must declare combos",
            raw.id
        );
        let combos_display = combos
            .iter()
            .map(|pattern| pattern.display.clone())
            .collect::<Vec<_>>()
            .join("/");
        let snippet = format!("{combos_display} -> {}", raw.description);
        Self {
            action,
            contexts,
            combos,
            snippet,
        }
    }

    fn matches(&self, key: &KeyEvent) -> Option<KeyAction> {
        self.combos
            .iter()
            .find(|pattern| pattern.matches(key))
            .map(|_| self.action)
    }
}

struct KeyPattern {
    matcher: CodeMatcher,
    required: KeyModifiers,
    allow_shift: bool,
    display: String,
}

impl KeyPattern {
    fn parse(spec: &str) -> Result<Self, String> {
        let display = spec.trim().to_string();
        if display.is_empty() {
            return Err("combo cannot be empty".into());
        }
        let mut tokens = display
            .split('+')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>();
        if tokens.is_empty() {
            return Err("combo must contain key".into());
        }
        let key_token = tokens.pop().unwrap();
        let matcher = CodeMatcher::from_token(key_token)?;
        let mut required = KeyModifiers::empty();
        for token in tokens {
            match token.to_lowercase().as_str() {
                "ctrl" | "control" => required |= KeyModifiers::CONTROL,
                "shift" => required |= KeyModifiers::SHIFT,
                "alt" => required |= KeyModifiers::ALT,
                other => {
                    return Err(format!("unsupported modifier '{other}'"));
                }
            }
        }
        let allow_shift = matcher.allows_extra_shift() && !required.contains(KeyModifiers::SHIFT);
        Ok(Self {
            matcher,
            required,
            allow_shift,
            display,
        })
    }

    fn matches(&self, key: &KeyEvent) -> bool {
        if !self.matcher.matches(&key.code) {
            return false;
        }
        if !modifiers_include(key.modifiers, self.required) {
            return false;
        }
        let extra = remove_modifiers(key.modifiers, self.required);
        if self.allow_shift {
            let tolerated = extra & !KeyModifiers::SHIFT;
            tolerated.is_empty()
        } else {
            extra.is_empty()
        }
    }
}

#[derive(Clone)]
enum CodeMatcher {
    Literal(KeyCode),
    Alpha(char),
}

impl CodeMatcher {
    fn from_token(token: &str) -> Result<Self, String> {
        let normalized = token.to_lowercase();
        let matcher = match normalized.as_str() {
            "tab" => CodeMatcher::Literal(KeyCode::Tab),
            "backtab" | "shift+tab" => CodeMatcher::Literal(KeyCode::BackTab),
            "enter" => CodeMatcher::Literal(KeyCode::Enter),
            "esc" | "escape" => CodeMatcher::Literal(KeyCode::Esc),
            "left" => CodeMatcher::Literal(KeyCode::Left),
            "right" => CodeMatcher::Literal(KeyCode::Right),
            "up" => CodeMatcher::Literal(KeyCode::Up),
            "down" => CodeMatcher::Literal(KeyCode::Down),
            other => {
                if other.len() == 1 {
                    CodeMatcher::Alpha(other.chars().next().unwrap())
                } else {
                    return Err(format!("unsupported key '{token}'"));
                }
            }
        };
        Ok(matcher)
    }

    fn matches(&self, code: &KeyCode) -> bool {
        match (self, code) {
            (CodeMatcher::Literal(expected), actual) => actual == expected,
            (CodeMatcher::Alpha(expected), KeyCode::Char(actual)) => {
                actual.to_ascii_lowercase() == *expected
            }
            _ => false,
        }
    }

    fn allows_extra_shift(&self) -> bool {
        matches!(
            self,
            CodeMatcher::Alpha(_) | CodeMatcher::Literal(KeyCode::BackTab)
        )
    }
}

impl RawAction {
    fn into_action(self) -> KeyAction {
        match self {
            RawAction::Save => KeyAction::Save,
            RawAction::Quit => KeyAction::Quit,
            RawAction::ResetStatus => KeyAction::ResetStatus,
            RawAction::TogglePopup => KeyAction::TogglePopup,
            RawAction::EditComposite => KeyAction::EditComposite,
            RawAction::FieldStep { delta } => KeyAction::FieldStep(delta),
            RawAction::SectionStep { delta } => KeyAction::SectionStep(delta),
            RawAction::RootStep { delta } => KeyAction::RootStep(delta),
            RawAction::ListAddEntry => KeyAction::ListAddEntry,
            RawAction::ListRemoveEntry => KeyAction::ListRemoveEntry,
            RawAction::ListMove { delta } => KeyAction::ListMove(delta),
            RawAction::ListSelect { delta } => KeyAction::ListSelect(delta),
        }
    }
}

static KEYMAP: LazyLock<Vec<KeyBinding>> = LazyLock::new(|| {
    let raw_entries: Vec<RawEntry> =
        serde_json::from_str(keymap_source!()).expect("invalid keymap/default.keymap.json");
    raw_entries.into_iter().map(KeyBinding::from_raw).collect()
});

pub(super) fn classify_key(key: &KeyEvent) -> Option<KeyAction> {
    KEYMAP.iter().find_map(|binding| binding.matches(key))
}

pub(super) fn help_text(context: KeymapContext) -> Option<String> {
    let snippets = KEYMAP
        .iter()
        .filter(|binding| binding.contexts.contains(&context))
        .map(|binding| binding.snippet.clone())
        .collect::<Vec<_>>();
    if snippets.is_empty() {
        None
    } else {
        Some(snippets.join(" • "))
    }
}

fn modifiers_include(actual: KeyModifiers, required: KeyModifiers) -> bool {
    actual.contains(required)
}

fn remove_modifiers(actual: KeyModifiers, required: KeyModifiers) -> KeyModifiers {
    KeyModifiers::from_bits_truncate(actual.bits() & !required.bits())
}
