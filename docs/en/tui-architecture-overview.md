# TUI Architecture & Keymap Overview

This document is a concise guide to the **TUI runtime**, its **UI concepts**
(main page, sections, child-sections, entries, fields, overlays, popups, help
overlay), and the **keymap** that drives keyboard navigation.

It is designed for engineers who need to:

- Understand how the current implementation actually works.
- Safely extend/refactor the TUI, overlays, or key bindings.
- Explain these concepts to other teammates.

The content reflects the **current codebase** under `src/tui` and
`keymap/default.keymap.json`.

---

## 1. Where the TUI fits in the pipeline

At a high level, the TUI is a frontend that consumes a prepared
`FrontendContext` from the core pipeline.

**Key code:**

- `src/core/pipeline.rs` – `SchemaPipeline`
- `src/core/frontend.rs` – `Frontend`, `FrontendContext`
- `src/tui/session.rs` – `TuiFrontend`
- `src/tui/app/schema_ui.rs` – `SchemaUI` builder

### 1.1 Data & control flow

```text
JSON Schema + defaults (serde_json::Value)
    │
    ▼
core::SchemaPipeline
    - schema_with_defaults
    - validator_for
    - build_ui_ast
    │
    ▼
FrontendContext { ui_ast, validator, initial_data, schema }
    │
    ▼
TuiFrontend::run(ctx)
    - form_schema_from_ui_ast → FormSchema
    - FormState::from_schema_with_palette
    │
    ▼
App::run()
    - input loop
    - overlays, popups, help overlay
    │
    ▼
View (tui::view::draw)
    - main page, overlay, popup, help overlay
```

Simplified Rust-like pseudocode (from `SchemaUI`):

```rust
fn run_tui(schema: Value, defaults: Option<Value>) -> Result<Value> {
    let pipeline = SchemaPipeline::new(schema)
        .with_title(Some("Title".into()))
        .with_defaults(defaults);

    let frontend = TuiFrontend { options: UiOptions::default() };
    pipeline.run_with_frontend(frontend)
}
```

From here on, we focus on **TuiFrontend**, **FormState**, the `App` runtime, and
the keymap.

---

## 2. Core UI concepts

This section gives precise meanings to the TUI terms you will see in the code
and docs.

### 2.1 Main page (root form)

The **main page** is the root form view that shows:

- Root tabs (one per top-level schema group)
- Sections and child-sections within the active root
- Fields inside the active section
- Inline errors and a footer status line

**Key code:**

- `src/tui/state/form_state.rs` – `FormState`
- `src/tui/state/section.rs` – `SectionState`
- `src/tui/model/layout` – `FormSchema`, `FormSection`
- `src/tui/view/components/body.rs` – `render_body`
- `src/tui/view/frame.rs` – `UiContext`, `draw`

The runtime representation is `FormState`:

```rust
pub struct FormState {
    pub roots: Vec<RootSectionState>,
    ui: UiStores,
}

pub struct RootSectionState {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub sections: Vec<SectionState>,
}

pub struct SectionState {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub path: Vec<String>,       // breadcrumb-like path
    pub depth: usize,            // 0 = top-level, >0 = child-section
    pub fields: Vec<FieldState>,
    pub scroll_offset: usize,
}
```

`UiStores` tracks focus within the main page:

```rust
pub struct UiStores {
    pub root: RootTabsStore,
    pub sections: SectionTabsStore,
    pub fields: FieldListStore,
}
```

**Mapping from schema:**

- Top-level object properties → `RootSectionState` (root tabs).
- Nested objects → `SectionState` (flattened into a per-root list, with `depth`
  to indicate nesting).
- JSON Pointers from the schema become `pointer` fields in `FieldSchema` and
  `FieldState`, used for validation and error mapping.

**Rendered by:** `render_body`, which uses the active root, section, and field
focus to draw the visible part of the main page.

### 2.2 Sections & child-sections

**Sections** are logical groups of fields within a root (e.g. "Metadata",
"HTTP", "TLS"). **Child-sections** are nested sections inside a section.

At schema-adaptation time (`tui::model::form_schema_from_ui_ast`), a tree of
`FormSection` is produced. At runtime, this tree is flattened into a vector of
`SectionState` while preserving `depth` and `path`:

```rust
impl SectionState {
    pub fn collect(
        section: &FormSection,
        depth: usize,
        palette: &Arc<ComponentPalette>,
        acc: &mut Vec<SectionState>,
    ) {
        // Create a SectionState for this section
        let fields = section
            .fields
            .iter()
            .cloned()
            .map(|schema| FieldState::from_schema_with_palette(schema, Arc::clone(palette)))
            .collect();
        acc.push(SectionState {
            id: section.id.clone(),
            title: section.title.clone(),
            description: section.description.clone(),
            path: section.path.clone(),
            depth,
            fields,
            scroll_offset: 0,
        });

        // Recursively add child sections
        for child in &section.children {
            SectionState::collect(child, depth + 1, palette, acc);
        }
    }
}
```

- Parent/child relationships are captured by `depth` and `path`.
- The view layer uses these to render indentation and breadcrumbs for
  child-sections.

### 2.3 Fields

A **field** is an actual editable control – string, number, enum, composite,
list, key/value map, etc.

**Key code:**

- `src/tui/state/form_state.rs` – `FieldState`
- `src/tui/state/field/components` – `FieldComponent` impls
- `src/tui/view/components/body.rs` – field row rendering

Runtime representation:

```rust
pub struct FieldState {
    pub schema: FieldSchema,
    pub(crate) component: Box<dyn FieldComponent>,
    pub dirty: bool,
    pub error: Option<String>,
}
```

The concrete `component` encodes behaviour for different schema shapes:

- Primitives – text/numeric editors.
- Enums – popup-backed selection.
- Composites – `oneOf`/`anyOf` single variant.
- Composite lists – arrays of composites ("entries").
- Key/value – map-like editors.
- Scalar arrays – arrays of simple types.

**Schema mapping:** a `FieldSchema` references a resolved `SchemaObject` and its
JSON pointer; that pointer is used to:

- Build values from `FormState` (`try_build_value`).
- Attach validator errors to a specific field.
- Show pointers in the help overlay error list.

### 2.4 Entries (collection entries)

An **entry** is a single element in a repeatable field:

- Items of a composite list (`CompositeListState`), such as an array of objects
  or `anyOf`-wrapped items.
- Key/value entries in a map.
- Items in a scalar array.

**Key code:**

- `src/tui/state/composite/composite_list.rs` – `CompositeListState`
- `src/tui/state/field/components/composite_list.rs` – list field component
- `src/tui/app/runtime/list_ops.rs` – list operations
- `src/tui/app/popup.rs` – variant selector popups for entries

`CompositeListState` maintains things like:

- `entries` – per-entry state (variant, summary, nested value).
- `selected_index` – which entry is focused.
- Operations: add/remove/move/select entries.

These operations are surfaced to the runtime via `FormCommand` and ultimately
hooked to key bindings like `Ctrl+N`, `Ctrl+D`, `Ctrl+Left/Right`,
`Ctrl+Up/Down` (see the keymap JSON and list ops section below).

### 2.5 Popups

A **popup** is a small, centered list used for choosing a value:

- Enum choices
- Boolean toggles
- Composite variant choices
- Composite list entry variant choices

**Key code:**

- `src/tui/app/popup.rs` – `PopupState`, `PopupOwner`
- `src/tui/app/runtime/mod.rs` – `popup: Option<AppPopup>`, `handle_popup_key`
- `src/tui/view/components/popup.rs` – `render_popup`

The runtime holds:

```rust
struct AppPopup {
    owner: PopupOwner,
    state: PopupState,
}

pub struct App {
    // ...
    popup: Option<AppPopup>,
}
```

`PopupOwner` tells the runtime how to apply a selection:

- `Root` – directly mutates `FormState`.
- `Composite` – mutates the active overlay.
- `VariantSelector { .. }` – picks a variant for a composite list entry.

Popups are **modal**: while a popup is open, `App::handle_key` routes events
through `handle_popup_key` first, independent of keymap context.

### 2.6 Overlays

An **overlay** is a full-screen editor used for nested structures:

- Editing a composite field (oneOf/anyOf object, polymorphic union).
- Editing a single entry in a composite list.
- Editing key/value entries or scalar arrays when they deserve their own form.

**Key code:**

- `src/tui/app/runtime/overlay` – overlay runtime logic
- `src/tui/app/runtime/mod.rs` – `overlay_stack`
- `src/tui/view/components/overlay.rs` – `render_composite_overlay`
- `src/tui/view/frame.rs` – `CompositeOverlay`, layering in `draw`

The runtime maintains a stack of overlays:

```rust
pub struct App {
    // ...
    overlay_stack: Vec<CompositeEditorOverlay>,
}
```

Each overlay wraps its own `FormState` plus metadata:

```rust
pub struct OverlayState {
    pub field_pointer: String,
    pub field_label: String,
    pub host: OverlayHost,
    pub level: usize,
    pub target: CompositeOverlayTarget,
    pub session: OverlaySession,
    // ...
}
```

- `OverlayHost::RootForm` means the overlay is editing a field from the main
  page.
- `OverlayHost::Overlay { parent_level }` handles overlays opened from overlays.
- `CompositeOverlayTarget` distinguishes field-wide editing vs per-entry
  editing.

**Lifecycle (simplified):**

```text
Form focus ──Ctrl+E──▶ try_open_composite_editor
                          │
                          ▼
                 CompositeEditorOverlay::new
                          │ setup_overlay_validator
                          ▼
                   overlay FormState + StatusLine
                          │
        (InputRouter + Keymap reused inside overlay)
                          │
                          ▼
                     Ctrl+S ──▶ save_active_overlay (stay open)
                          │
                          ▼
                     Esc/Q ──▶ close_active_overlay (commit)
```

Overlays reuse the global keymap and validator infrastructure but apply it to
nested `FormState` instances.

### 2.7 Help overlay

The **help overlay** is a top-most layer that shows:

- A paginated list of keyboard shortcuts (grouped by keymap context).
- A summary of current field errors (JSON pointer + truncated message).

**Key code:**

- `src/tui/app/runtime/mod.rs` – `HelpOverlayState`, `toggle_help_overlay`,
  `handle_help_overlay_key`
- `src/tui/view/frame.rs` – `HelpOverlayRender`, `UiContext`
- `src/tui/view/components/help.rs` – `render_help_overlay`
- `src/tui/state/form_state.rs` – `error_entries()`

Runtime state:

```rust
pub struct App {
    // ...
    help_overlay: Option<HelpOverlayState>,
}

struct HelpOverlayState {
    pages: Vec<Vec<String>>, // each page is a list of lines
    page: usize,             // current page index
}
```

Key handling:

```rust
fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
    if key.kind != KeyEventKind::Press { return Ok(()); }

    if self.handle_help_overlay_key(&key) { return Ok(()); }
    if self.handle_popup_key(key)? { return Ok(()); }
    // then overlay / main page handling
}

fn handle_help_overlay_key(&mut self, key: &KeyEvent) -> bool {
    let Some(state) = self.help_overlay.as_mut() else { return false; };
    match key.code {
        KeyCode::Esc => { self.help_overlay = None; true }
        KeyCode::Tab => { /* next page */ true }
        KeyCode::BackTab => { /* previous page */ true }
        _ => false,
    }
}
```

In the view layer, `UiContext` includes an optional `HelpOverlayRender`, and
`frame::draw` renders it **last** so it sits above the main page, overlays, and
popups.

---

## 3. Keymap model & contexts

The keymap is defined in JSON and parsed at startup.

**Key code:**

- `keymap/default.keymap.json` – keymap data
- `src/tui/app/keymap.rs` – parsing, `KeymapContext`, `KeymapStore`
- `src/tui/app/options.rs` – wiring into `UiOptions`
- `src/tui/app/input.rs` – `KeyAction`, `KeyBindingMap`, `InputRouter`

### 3.1 JSON structure

Each entry in `default.keymap.json` looks like this:

```json
{
  "id": "list.move.up",
  "description": "Move entry up",
  "contexts": ["collection", "overlay"],
  "dispatch": true,
  "action": { "kind": "listMove", "delta": -1 },
  "combos": ["Ctrl+Up"]
}
```

Fields:

- `id` – stable identifier for the binding (used in docs/logs).
- `description` – human-readable text used in help messages.
- `contexts` – semantic scopes (`"default"`, `"collection"`, `"overlay"`,
  `"help"`, `"text"`, `"numeric"`).
- `dispatch` – optional; `false` means the binding is help-only and must not
  intercept the real key event.
- `action` – tagged union that deserializes to `RawAction` → `KeyAction`.
- `combos` – list of textual combos (e.g. `"Ctrl+Shift+Tab"`).

`KeymapStore` parses these entries, converts them into `KeyBinding` instances
with parsed `KeyPattern`s, and exposes two main APIs:

```rust
pub fn classify(&self, key: &KeyEvent) -> Option<KeyAction>;
pub fn help_text(&self, context: KeymapContext) -> Option<String>;
```

### 3.2 Keymap contexts

`KeymapContext` represents semantic groups for help text:

```rust
pub enum KeymapContext {
    Default,
    Collection,
    Overlay,
    Help,
    TextInput,
    NumericInput,
}
```

They are **not** used to filter which keys fire; instead they:

- Determine which bindings contribute to the footer help string.
- Drive which bindings appear in the help overlay for each page.
- Allow help-only field editor hints to live in the same JSON source as
  dispatching shortcuts.

The runtime picks one or more contexts based on focus:

```rust
fn current_help_text(&self) -> Option<String> {
    if !self.options.show_help { return None; }
    let contexts = self.current_help_contexts();
    self.keymap_store.help_text_for_contexts(&contexts)
}
```

Semantic meaning:

- **Default** – app-wide navigation and editing when not in a special list
  context and not inside an overlay.
- **Collection** – list operations (add/remove/select/move entries) when a
  collection-like field is focused on the main page.
- **Overlay** – actions while inside any overlay (including list operations for
  collections inside overlays).
- **Help** – modal help-overlay controls such as close, page switching,
  shortcuts scrolling, and horizontal error-text scrolling.
- **TextInput** – help-only hints for `string` / `json` text editing
  (`Left/Right`, `Home/End`, delete, undo/redo, `Ctrl+W`).
- **NumericInput** – help-only hints for `integer` / `number` editors
  (`Left/Right` stepper, `Shift+Left/Right` fast-stepper, delete, undo/redo).

### 3.3 From KeyEvent to AppCommand/FormCommand

Event flow (simplified):

```text
KeyEvent (crossterm)
    │
    ▼
InputRouter::classify
    │  uses KeymapStore::classify (KeyPattern match)
    ▼
KeyAction
    │
    ▼
KeyBindingMap::resolve
    │
    ▼
CommandDispatch::{Form(FormCommand), App(AppCommand)}
    │
    ├─ FormCommand  → FormEngine & FormState
    └─ AppCommand   → App::handle_app_command / handle_overlay_app_command
```

The distinction:

- `FormCommand` – mutates `FormState` (focus changes, value edits, list ops).
- `AppCommand` – controls overlays, popups, save/quit, help overlay, etc.

Because contexts affect **only help**, the classification pipeline stays simple
and predictable.

---

## 4. Concept → code → keymap mapping

The following table summarizes how the major UI concepts map to code and keymap
contexts:

| Concept       | Schema / layout                                 | Runtime structures                                            | View components                           | Typical contexts |
| ------------- | ----------------------------------------------- | ------------------------------------------------------------- | ----------------------------------------- | ---------------- |
| Main page     | Root object properties → roots and sections     | `FormState`, `RootSectionState`, `SectionState`, `FieldState` | `components::body`, `frame::draw`         | `Default`        |
| Section       | Nested objects / groups                         | `SectionState { depth, path }`                                | Section headers in `body`                 | `Default`        |
| Child-section | Nested section with `depth > 0`                 | Same as section, but deeper `depth`                           | Indented/ breadcrumb section header       | `Default`        |
| Field         | Leaf schema nodes (string, enum, composite...)  | `FieldState` + `FieldComponent`                               | Field rows in `body`                      | `Default`        |
| Entry         | Array items / map entries / composite list item | `CompositeListState`, key/value state, scalar array state     | Entries header/strip + inline summaries   | `Collection`     |
| Popup         | Enum/variant selector                           | `PopupState`, `AppPopup`, `PopupOwner`                        | `components::popup`                       | (modal, no ctx)  |
| Overlay       | Composite editor / nested form                  | `CompositeEditorOverlay`, `overlay_stack`, nested `FormState` | `components::overlay`, `CompositeOverlay` | `Overlay`        |
| Help overlay  | n/a                                             | `HelpOverlayState` in `App`                                   | `components::help`, `HelpOverlayRender`   | All (by context) |

This table is a good starting point when you onboard a new engineer or review a
refactor.

---

## 5. Refactor & extension guide

This section gives a concrete, non-exhaustive checklist for typical TUI changes.

### 5.1 Adding or changing a key binding

1. **Add or update** an entry in `keymap/default.keymap.json`:

   ```json
   {
     "id": "help.toggle",
     "description": "Toggle help overlay",
     "contexts": ["default", "collection", "overlay"],
     "action": { "kind": "showHelp" },
     "combos": ["Ctrl+?"]
   }
   ```

2. **Ensure** the action is supported in `RawAction` → `KeyAction` mapping
   (`src/tui/app/keymap.rs`):

   ```rust
   #[serde(tag = "kind", rename_all = "camelCase")]
   enum RawAction {
       Save,
       Quit,
       ResetStatus,
       TogglePopup,
       EditComposite,
       ShowHelp,
       // ...
   }

   impl RawAction {
       fn into_action(self) -> KeyAction {
           match self {
               RawAction::ShowHelp => KeyAction::ShowHelp,
               // ...
           }
       }
   }
   ```

3. **Map** the `KeyAction` to a `CommandDispatch` in `KeyBindingMap::resolve`
   (`src/tui/app/input.rs`):

   ```rust
   KeyAction::ShowHelp => self
       .bindings
       .get(&KeyActionDiscriminant::ShowHelp)
       .cloned()
       .unwrap_or(CommandDispatch::App(AppCommand::ShowHelp)),
   ```

4. **Handle** the `AppCommand` in `App::handle_app_command` or
   `handle_overlay_app_command` (`src/tui/app/runtime/mod.rs`).

### 5.2 Adding a new overlay-based editor

Suppose you want a new overlay type for a complex field.

High-level steps:

1. **Model the field** in the TUI schema adapter:
   - Extend `FieldKind` / `FieldSchema` (if necessary) in `tui::model`.
   - Ensure `form_schema_from_ui_ast` maps the relevant `UiNodeKind` into the
     new field kind.

2. **Add a `FieldComponent`** implementation in `src/tui/state/field/components`
   and wire it into `FieldState::from_schema`.

3. **Define overlay state**:
   - Extend `CompositeOverlayTarget` / `OverlayHost` in
     `src/tui/app/runtime/overlay`.
   - Add fields to the overlay session if extra metadata is required.

4. **Open the overlay**:
   - Update `try_open_composite_editor` or add a new helper in
     `overlay/app/open.rs` to instantiate the new overlay from a focused field.

5. **Render it**:
   - Extend `CompositeOverlay` / `render_composite_overlay` in
     `src/tui/view/components/overlay.rs` if the layout differs.

6. **Wire commands**:
   - Reuse existing `AppCommand`/`FormCommand` where possible (KISS).
   - Only introduce new commands if the behaviour cannot be expressed via
     list/field edits.

### 5.3 Keeping docs and code in sync

When you change the TUI or keymap:

- Update:
  - `README.md` and `README.ZH.md` (high-level snapshot).
  - `docs/en/structure_design.md` / `docs/zh/structure_design.md`
    (architecture/deep dive).
  - `docs/en/cli_usage.md` / `docs/zh/cli_usage.zh.md` (CLI behaviour).
  - This overview file if concepts or contexts change.
- Avoid duplicating low-level detail across many docs; keep **flow diagrams and
  refactor notes** centralized here and in `structure_design.md`.

---

## 6. Related documents

- `README.md` – project overview and architecture snapshot.
- `docs/en/structure_design.md` – full architecture and pipeline design.
- `docs/en/cli_usage.md` – CLI usage, flags, and examples.

This overview is meant to be the **short, accurate entrypoint** for new and
existing contributors working on the TUI and its keyboard interactions.
