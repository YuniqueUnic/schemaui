# Composite/KeyValue Roadmap

This document synthesizes the latest requirements for SchemaUI’s dynamic form
runtime. It is meant to brief the tech-lead and serve as a hand-off for
engineers who will implement the next set of features. All file paths are
workspace-relative.

---

## 1. Current Architecture (as of 2025‑11‑10)

| Layer        | Responsibility                                                                      | Key files                                                                                |
| ------------ | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Parser       | Translate JSON Schema into `FormSchema`/`FieldSchema`, detect composites and arrays | `src/domain/parser.rs`, `src/domain/schema.rs`                                           |
| Form State   | Hold `SectionState`, `FieldState`, nested `CompositeState` / `CompositeListState`   | `src/form/state.rs`, `src/form/field.rs`, `src/form/composite.rs`, `src/form/section.rs` |
| Runtime      | Event loop, input classification, overlay management, validation                    | `src/app/input.rs`, `src/app/runtime.rs`, `src/app/popup.rs`                             |
| Presentation | Render sections, fields, status/actions bars, overlays                              | `src/presentation/components.rs`, `src/presentation/view.rs`                             |
| Docs         | README + internal report                                                            | `README.md`, `report-composite.md` (this file)                                           |

Recent upgrades (Nov 2025) already added:

- Composite overlay with list sidebar, Ctrl+E editing, Ctrl+N/D/←/→/↑/↓ list
  management.
- Default seeding via `FieldState::seed_value` and `FormState::seed_from_value`.
- Section scrolling, improved footer/actions bar, inline error wrapping.

---

## 2. New Requirements Overview

| Theme                                      | Goal                                                                                                                  | Primary Deliverables                                                                                                                                      |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **AdditionalProperties / KeyValue Editor** | Provide structure editing for schema-defined maps so users never type raw JSON                                        | New `FieldKind::KeyValue` (or reuse existing `FieldKind::Json` with metadata), `KeyValueState`, key validator, value editor that reuses composite overlay |
| **Overlay-Level Validation**               | Run `jsonschema::Validator` after each field edit within overlays to surface errors immediately                       | Local validator instance + error mapper + visual cues identical to main form                                                                              |
| **Documentation / Help**                   | Keep README + in-app actions accurate per context (standard fields, composite lists, key/value editing)               | README shortcuts table, contextual actions ribbon, updated HELP_TEXT                                                                                      |
| **Section & OneOf/AnyOf UX**               | Clarify section rules (scalar → General, object/composite → dedicated section) and show intuitive OneOf/AnyOf headers | Parser tweak + UI layout similar to spec in TODO                                                                                                          |

---

## 3. AdditionalProperties Key/Value Editor

### 3.1 Data Model

- **New FieldKind**: extend `src/domain/schema.rs` with
  `FieldKind::KeyValue(Box<FieldKind>)` where the inner kind describes the value
  schema. When schema has `additionalProperties: { anyOf: [...] }`, wrap it in a
  composite-aware kind.
- **FieldState**: add `FieldValue::KeyValue(KeyValueState)` where
  `KeyValueState` holds:
  ```rust
  pub struct KeyValueState {
      pub entries: Vec<KeyValueEntry>,
      pub selected: usize,
  }

  pub struct KeyValueEntry {
      pub key_field: FieldState,              // handles string + validation
      pub value_field: FieldState,            // may be composite / list / scalar
      pub pointer: String,                    // e.g. "/featureFlags/production"
  }
  ```
- Keys follow JSON Schema rules: use `FieldState` for the key itself so we can
  reuse `field.seed_value` and validation logic. For uniqueness, maintain a
  `HashSet` on `KeyValueState` and poke `FieldCoercionError` if duplicates
  appear.

### 3.2 Parser Changes

1. When `ObjectValidation.additional_properties` is present:
   - Build `FieldSchema` with `FieldKind::KeyValue(Box<FieldKind>)`.
   - Use the current property `path_prefix` so values insert under
     `/section/field/key`.
   - Persist metadata (pattern, minLength, etc.) onto the key schema.

2. For `anyOf`/`oneOf` inside `additionalProperties`, rely on existing
   `CompositeField` generation so value editor is consistent.

### 3.3 Runtime & UI

- **List interactions**: reuse the existing list shortcuts (Ctrl+N/D/←/→/↑/↓),
  but operate on `KeyValueState.entries`.
- **Overlay**: pressing Ctrl+E on a KeyValue field opens the composite overlay:
  - Left sidebar shows keys in order (e.g. `#1 production`, `#2 staging`).
  - Right panel renders key editor on top (string input) followed by value
    editor (scalar/composite). Example ASCII:
    ```
    KeyValue Overlay: featureFlags
    ┌ Sidebar (#1 production) (#2 beta-users) ┐ ┌ Key Editor ┐
    │ Ctrl+N add • Ctrl+D delete             │ │ Key: [production     ] │
    └────────────────────────────────────────┘ │ Value: (oneOf combo)  │
    ```
- **Serialization**: `FieldState::current_value` for KeyValue iterates entries,
  obtains `key_field.current_value()` (string) and `value_field.current_value()`
  (Value), then builds a `serde_json::Map`.

### 3.4 Example Code Snippets

```rust
// src/form/field.rs (construction)
FieldKind::KeyValue(inner) => FieldValue::KeyValue(
    KeyValueState::new(&schema.pointer, inner.as_ref(), schema.default.as_ref())
);

// src/form/composite.rs or new keyvalue module
impl KeyValueState {
    pub fn insert_entry(&mut self) -> usize { /* reuse list logic */ }
    pub fn remove_entry(&mut self, index: usize) -> bool { /* ... */ }
    pub fn try_build_object(&self) -> Result<Value, FieldCoercionError> { /* ... */ }
}
```

---

## 4. Overlay-Level Validation

### 4.1 Triggers

- Run validation **after each edit** (called from `FieldState::handle_key` or
  list operations). For text input we debounce at “editing finished” events:
  e.g. Enter, blur, or when popup closes.

### 4.2 Implementation

1. **Local Validator**: overlay already stores a `CompositeEditorSession` with
   its own `FormState`. Add `validator: Validator` (precompiled for the variant)
   so overlay can call `validator.iter_errors(&value)`.
2. **Error Mapping**: reuse `FormState::set_error(pointer, message)` inside
   overlay `FormState`. For entries inside KeyValue/CompositeList, sanitize
   pointer (e.g. `/featureFlags/production/type`).
3. **UI Feedback**: `render_body` already draws inline error lines. After
   validation, the red wrapped messages appear just like the main form.
4. **Save Semantics**: On Ctrl+S we still run global validation, but even if
   overlay has errors we allow exit (per requirement). However, we skip writing
   to temp file until global validation passes.

Pseudo-code:

```rust
fn overlay_validate(&mut self) {
    if let Some(editor) = self.composite_editor.as_mut() {
        let mut form = editor.form_state_mut();
        match form.try_build_value() {
            Ok(value) => {
                editor.validator.reset_errors();
                for err in editor.validator.iter_errors(&value) {
                    form.set_error(&err.instance_path.to_string(), err.to_string());
                }
            }
            Err(err) => form.set_error(&err.pointer, err.message),
        }
    }
}
```

Call `overlay_validate()` whenever:

- `FieldState::handle_key` returns true inside overlay mode.
- List/KeyValue operations change entries.
- Popup selection updates a value.

---

## 5. Section & OneOf/AnyOf UI

### 5.1 Section Rules

- Scalars (`type: string/integer/...`) stay in the default “General” section.
- Objects with `properties` or composite semantics become their own section.
  Parser already checks `has_composite_subschemas`; expand logic so
  `additionalProperties` fields create a section named after the parent key
  (e.g. `featureFlags`).

### 5.2 UI Layout

- Section header shows tabs as plain titles (no `[id]` suffix).
- For OneOf:
  ```
  Section: DataStore
  OneOf: [ SQL Database | NoSQL Database ]
  ──────────────────────────────────────────
  [Variant fields render here]
  ```
- For AnyOf, show chips + “+ Add variant” button that triggers multi-select
  popup. Each chosen variant renders beneath the control.

Implementation pointers:

- `render_fields` should render a small control block before the child form when
  `FieldKind::Composite` is focused. We can reuse existing
  `FieldValue::Composite` summary but convert it to actual UI controls
  (tabs/chips).
- Shortcut: Enter still opens popup; we can also add `Ctrl+[` / `Ctrl+]` to
  cycle OneOf variants later if needed.

---

## 6. Documentation & Contextual Help

| Context          | Actions ribbon content                                  | Files to edit                                                      |
| ---------------- | ------------------------------------------------------- | ------------------------------------------------------------------ |
| Default form     | Existing navigation/save shortcuts                      | `src/presentation/components.rs`                                   |
| Composite list   | Add `Ctrl+N/D/←/→/↑/↓` hints                            | `src/app/runtime.rs` (HELP_TEXT), `src/presentation/components.rs` |
| Key/Value editor | Add `Ctrl+Enter` rename? (optional) plus list shortcuts | same as above                                                      |
| README           | New “Keyboard Shortcuts” table broken down by context   | `README.md`                                                        |
| Report           | Keep this document in sync                              | `report-composite.md`                                              |

Example README section:

```markdown
### Keyboard Shortcuts

| Context          | Keys            | Description                   |
| ---------------- | --------------- | ----------------------------- |
| Global           | Ctrl+S          | Save (run validation)         |
| Composite List   | Ctrl+N / Ctrl+D | Add / remove entry            |
| Key/Value Editor | Ctrl+E          | Open overlay for selected key |
```

---

## 7. Delivery Plan

| Step | Description                                                              | Main files                                                                                   | Expected Tests                                      |
| ---- | ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| 1    | Introduce `FieldKind::KeyValue`, parser wiring, `KeyValueState` skeleton | `src/domain/schema.rs`, `src/domain/parser.rs`, `src/form/field.rs`, `src/form/composite.rs` | `cargo check` + unit tests for `KeyValueState`      |
| 2    | UI & runtime for key list overlay (reuse composite overlay, add sidebar) | `src/app/runtime.rs`, `src/presentation/components.rs`, `src/app/input.rs`                   | Manual run with sample schema (featureFlags)        |
| 3    | Overlay validator (local `Validator`, inline errors)                     | `src/app/runtime.rs`, `src/form/state.rs`, `src/app/validation.rs`                           | Unit test covering error mapping, manual validation |
| 4    | Section/OneOf/AnyOf header refactor                                      | `src/domain/parser.rs`, `src/presentation/components.rs`                                     | Visual check with `main.rs` schema                  |
| 5    | Contextual help + README updates                                         | `src/presentation/components.rs`, `README.md`, this report                                   | Doc review                                          |
| 6    | End-to-end regression                                                    | `cargo check`, `cargo clippy`, `cargo test` (future)                                         | Ensure no regressions                               |

---

## 8. Sample Workflows (FeatureFlags)

1. User focuses `featureFlags` field (KeyValue).
2. Presses `Ctrl+N` → new entry `#1 key-1` appears; key input gains focus.
3. Types `production`, overlay validation runs; if pattern fails, inline red
   text shows immediately.
4. Presses `Ctrl+E` to edit value: overlay opens with key editor + value
   composite (AnyOf chips).
5. Chooses `boolean` variant from popup; toggles value.
6. Presses `Ctrl+S`: overlay validation + global validation run; status shows
   success or aggregated errors.

Data path:

```
JSON Schema -> FieldKind::KeyValue -> FieldState::KeyValueState
           -> FormState::try_build_value -> serde_json::Map -> Validator
```

---

## 9. Open Questions (Track in TODO-questions.md)

- Do we need persistence for user-defined key order (store in schema metadata)?
  A: yes, keep the order as user set
- Should key rename propagate pointers for nested composite states? A: yes. must
  sync to avoid json path issue.
- Will we support drag-and-drop ordering in the future (beyond keyboard)? A: no,
  no need to do it yet.

Once these decisions are locked, the implementation can proceed according to
Section 7. This report should be shared with the tech-lead for approval before
coding starts.

---

## 10. Implementation Status (2025-11-10)

Recent changes stitched the plan above into the codebase:

- **Key/Value editor (AdditionalProperties)**: `FieldKind::KeyValue` now
  represents map schemas and is backed by `KeyValueState` with full entry
  management (`src/domain/schema.rs`, `src/form/key_value.rs`). Runtime list
  shortcuts work for both composite lists and key/value maps, and Ctrl+E opens
  the dedicated overlay (`src/app/runtime.rs`).
- **Overlay-level validation**: Every overlay owns a scoped `jsonschema`
  validator. Keystrokes, popup selections, and entry operations trigger
  `validate_partial_form`, immediately surfacing errors without waiting for a
  global save (`src/app/runtime.rs`, `src/app/validation.rs`).
- **Dynamic help/actions**: The footer now shows context-aware shortcuts (base
  navigation, list/map management, overlay controls), and the README documents
  the same table to keep user-facing docs in sync (`README.md`).
- **Report/docs sync**: This section tracks the November 2025 delivery so the
  tech-lead can see which roadmap items already ship versus what remains (e.g.
  section/oneOf layout polish is still outstanding).
- **Section + OneOf/AnyOf UX**: scalar fields stay in `General`, while complex
  objects/composites (including `additionalProperties`) gain their own section
  tabs. Focused composite fields now render inline OneOf/AnyOf selectors with
  highlighted variants and inline “+ Add variant” hints, matching the UX mock
  described earlier (`src/domain/parser.rs`, `src/presentation/components.rs`).
