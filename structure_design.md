# schemaui Structure & Design Notes

This document explains how schemaui maps JSON Schema documents into a navigable TUI, the modules involved in each stage, and the guiding principles for validation, overlays, and keyboard ergonomics.

## 1. Guiding principles

- **Schema fidelity** – every construct that is legal in JSON Schema draft-07 (objects, arrays, `$ref`, `patternProperties`, `oneOf`/`anyOf`) must have a deterministic mapping to widgets.
- **Form-first thinking** – schema information is normalized into a `FormSchema`/`FormState` structure before any rendering happens. Drawing code never inspects raw schema JSON.
- **KISS & SOLID** – modules are cohesive (loader/resolver/layout, form state, runtime, presentation). Public APIs stay narrow (`SchemaUI`, `UiOptions`), while submodules communicate via well-defined structs.
- **Validation everywhere** – user edits run through `jsonschema::Validator` automatically. Overlays get their own validator derived from the focused subschema.
- **Keyboard determinism** – every key chord maps to a semantic `KeyAction`, then to a `FormCommand` or `AppCommand`. The same chord behaves identically whether the user is editing the main form or an overlay.

## 2. Pipeline overview

```text
JSON Schema
   │
   ▼
schema::loader          // parses draft-07 documents via schemars
   │
   ▼
schema::resolver        // resolves $ref (definitions + JSON Pointer references)
   │
   ▼
schema::layout::build_form_schema
   │   • assigns roots (top-level properties)
   │   • flattens nested objects into Section trees
   │   • maps instance types → FieldKind (string, enum, composite, key/value, etc.)
   ▼
FormSchema              // declarative description of roots/sections/fields
   │
   ▼
FormState               // runtime values, focus indices, dirty/error flags
   │
   ├─ FormEngine        // dispatches FormCommand & runs jsonschema validation
   └─ FieldState        // per-field value handling (text, lists, key/value, composites)
   ▼
App runtime             // event loop, overlays, persistence, keyboard routing
   │
   └─ presentation      // Ratatui components for tabs, body, overlays, status/footer
```

### Schema mapping highlights

| Schema construct | FieldKind / UI element |
| --- | --- |
| `type: string / integer / number` | Text input with numeric helpers (←/→ increments for numeric types) |
| `type: boolean` | Checkbox / toggle |
| `enum` | Select list (popup) |
| `oneOf` / `anyOf` | Composite selector + overlay editor |
| Arrays of scalars | Multi-line editor with summary lines |
| Arrays of composites | Repeatable list + overlay per entry |
| `patternProperties` / `additionalProperties` objects | Key/Value editor |
| `$ref` chains | Resolved eagerly by `SchemaResolver` and merged into the layout tree |

## 3. Form layer

- **FormSchema** (in `domain::schema`) stores `RootSection`, `FormSection`, and `FieldSchema`. `schema::layout` is the only module that constructs it.
- **FormState** (in `form::state`) keeps:
  - `roots`: vector of `RootSectionState` (flattened sections with depth info)
  - `root_index`, `section_index`, `field_index` for focus
  - `FieldState` instances with current value, dirty bit, and validation error
- **Navigation helpers**:
  - `focus_next_field` / `focus_prev_field` now wrap across sections and roots via `advance_section`.
  - `focus_next_section(delta)` cycles the flattened root order to keep Tab, Shift+Tab, and arrow keys consistent.
- **Validation**:
  - `FormEngine::dispatch` handles `FieldEdited { pointer }` commands by running `jsonschema::Validator` against the fully built JSON value and back-propagating errors to fields.
  - Overlay editors spin up their own `Validator` via `validator_for` so nested forms can be validated without leaving the overlay.

## 4. Runtime & presentation layering

- **`app::runtime::App`** orchestrates the program:
  - wraps terminal setup/teardown (`TerminalGuard`) and installs a panic hook that restores the TTY before delegating to `color-eyre`.
  - holds `StatusLine`, global errors, save/exit state, and the current popup/overlay.
  - draws the UI by passing `FormState` and optional overlay state into `presentation::draw`.
- **Overlay handling** (`app::runtime::overlay`):
  - `OverlaySession` wraps the specific editor type (composite, key/value, scalar array).
  - `CompositeEditorOverlay` tracks panel metadata, instructions, and validator state.
  - Overlay-specific `impl App` blocks live in `overlay.rs`, keeping `mod.rs` lean (< 800 lines).
- **List operations** (`app::runtime::list_ops`): add/remove/move/select logic is encapsulated so both root form and overlay reuse it without duplication.
- **Presentation** (`presentation::components`):
  - `sections.rs` renders root/section tabs.
  - `fields.rs` renders each section's field list, caret positioning, summaries, and meta lines.
  - `overlay.rs` draws the composite overlay shell with optional side panels.
  - `footer.rs` shows help/status/validation summaries.

## 5. Input & shortcuts

- **`app::input::InputRouter`** classifies `KeyEvent` into high-level `KeyAction` variants (field/section/root steps, save, quit, list operations, etc.).
- **`KeyBindingMap`** maps `KeyAction` → `CommandDispatch` (either a `FormCommand`, `AppCommand`, or raw input event). Custom keymaps can override defaults.
- **Command flow**:
  1. Read `crossterm::event::KeyEvent`.
  2. `InputRouter::classify` returns a `KeyAction`.
  3. `KeyBindingMap::resolve` maps into a `CommandDispatch`.
  4. `App::handle_key` either dispatches to `FormEngine`, executes an `AppCommand`, or routes raw input to the focused field.
- The shortcut table duplicated in the README lists the canonical chords.

## 6. Error handling & validation UX

- `color-eyre` is installed in `main` and a panic hook (in `terminal.rs`) ensures raw mode is disabled and the alternate screen is exited before printing stack traces.
- Status line helpers differentiate between "ready", "value updated", "pending exit", and validation error states.
- Popups and overlays always clear `status` messages to explain the current focus (e.g., list instructions, overlay save hints).

## 7. Tests & directory layout

- All tests live under `tests/` and mirror the source tree (`tests/form`, `tests/schema`, ...). They are `include!`-ed from the respective modules so private APIs remain testable.
- Current coverage includes:
  - `tests/form/key_value_tests.rs` → Unicode-safe summarization and truncation logic.
  - `tests/form/state_tests.rs` → cross-root navigation guarantees for fields & sections.
  - `tests/schema/layout_tests.rs` → schema-to-form mapping invariants (nested sections, composites, key/value, refs).
- Adding new tests follows the same pattern: place them under `tests/<module>/…` and include them via `concat!(env!("CARGO_MANIFEST_DIR"), ...)` in the corresponding module.

## 8. Working philosophy

- Keep each file under ~600 lines where practical (the largest runtime pieces were split into `overlay.rs`, `list_ops.rs`, and `mod.rs`).
- Prefer mature crates (jsonschema, ratatui, color-eyre, crossterm) over bespoke implementations.
- Small helper modules (`overlay.rs`, `list_ops.rs`, etc.) make it easy to reason about cross-cutting behaviors (popups, lists, overlays) without bloating `App`.
- Documentation should evolve with code; README targets users, while this file is the living design record for contributors.
