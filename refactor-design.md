# SchemaUI Refactor Design (vNext)

## 0. Background & Glossary

- **SchemaUI**: runtime TUI generator that consumes JSON Schema and produces an
  editable form.
- **Root section**: top-level property grouping (e.g., `runtime`, `metadata`).
  General is a pseudo-root for scalars.
- **Section tree**: hierarchical structure of sections; each node contains
  metadata and ordered children.
- **Field node**: leaf describing a renderable control plus validation metadata.
- **Overlay**: secondary form (modal) for editing composites, arrays, or
  key/value entries.
- **Command**: semantic action (e.g., `Command::NextSection`) triggered by
  input, processed by reducers.

## 1. Objectives & Constraints

1. Provide a predictable JSON Schema ➝ TUI pipeline that keeps structural
   fidelity (root sections, nested sections, `$ref`, `oneOf`, `anyOf`,
   `patternProperties`, etc.).
2. Guarantee validation after every edit: dirty fields are validated immediately
   through the `jsonschema` validator, errors are surfaced inline and aggregated
   on status bar.
3. Modularize the TUI so that sections, nested sections, overlays, and shortcut
   handling are loosely coupled and composable.
4. Keep the codebase maintainable: each module ≤ 600 LOC (hard cap 800), use
   existing crates (`schemars`, `jsonschema`, `ratatui`, `crossterm`).
5. Usability-first keyboard scheme inspired by nano/helix/vim but approachable
   for “plain” users (no chord gymnastics required).

Non-goals: redesigning persistence APIs, adding web UI, or changing jsonschema
versions.

## 2. Pain Points in Current Code

| Area       | Issue                                                                                                       |
| ---------- | ----------------------------------------------------------------------------------------------------------- |
| Parser     | Flattens nested sections, loses parent-child ordering, `$ref` contexts break inside composite overlays.     |
| Form state | Single list of sections cannot express nested navigation, no abstraction for root vs child tabs.            |
| UI         | Tabs widget assumes single row; shortcuts clash (Ctrl+[ is Esc), overlay shares state logic with root form. |
| Validation | Only triggered on demand (unless auto_validate); errors sometimes detach from fields.                       |
| Input      | Hard-coded shortcuts couple keys to actions; no indirection for mode changes or platform differences.       |

## 3. Target Architecture Overview

```
┌──────────┐
│ JSON File│
└──┬───────┘
   │ load (serde_json)
┌──▼────────────┐
│ Schema Loader │  (schemars::RootSchema)
└──┬────────────┘
   │ normalize (resolve $ref, inline composites, annotate metadata)
┌──▼────────────┐
│ Layout Graph  │  (SectionTree, FieldNode, VariantNode)
└──┬────────────┘
   │ hydrate defaults & validators
┌──▼────────────┐
│ Form State    │  (RootState → SectionState → FieldState)
└──┬────────────┘
   │ render (ratatui)
┌──▼────────────┐
│ View Layer    │  (SectionsPane, FieldsPane, OverlayPane, StatusPane)
└──┬────────────┘
   │ interact (crossterm events, InputRouter)
┌──▼────────────┐
│ Action Bus    │  (Commands → Reducers → State)
└───────────────┘
```

## 4. JSON Schema ➝ TUI Pipeline

### 4.1 Loader & Normalizer

1. **Source ingestion**: accept JSON Value or file path. Use `schemars` to
   deserialize into `RootSchema`.
2. **Reference resolver**: build a `SchemaArena` that stores every resolved
   schema node with a stable `SchemaId`. `$ref` uses arena IDs; composites keep
   pointer to arena.
3. **Metadata annotator**: unify `title`, `description`, `default`, `x-ui`
   hints. Provide fallback heuristics (snake_case ➝ Title Case).
4. **Tree builder**:
   - Determine root sections from top-level `properties` (General pseudo-section
     if scalar leaves exist).
   - Each object schema becomes
     `SectionNode { id, title, path, parent, children }`.
   - Leaves convert to `FieldNode` (includes `kind`, `validators`, `ui hints`).
5. **Ordering**: preserve `IndexMap` iteration order, carry an `ordinal` so
   nested merges respect JSON schema order.

### 4.2 Field Typing

| JSON Schema pattern                          | FieldKind                                                                  | Notes                         |
| -------------------------------------------- | -------------------------------------------------------------------------- | ----------------------------- |
| `type: string                                | integer                                                                    | number                        |
| `enum` or `const`                            | Enum/Toggle                                                                | Provide `options`, `default`. |
| `array` of scalar                            | `RepeatableScalar` with overlay editor.                                    |                               |
| `array` of enum                              | Multi-select.                                                              |                               |
| `array` of object                            | `RepeatableComposite`.                                                     |                               |
| `oneOf`/`anyOf`                              | `CompositeField { mode, variants[] }`, variant schema keeps arena context. |                               |
| `additionalProperties` / `patternProperties` | `KeyValueField`.                                                           |                               |

## 5. Validation Flow

1. **Validator registry**: compile `jsonschema::Compiled` once per root schema;
   pass shared `Arc`.
2. **Field-level validation**:
   - On each edit commit (Enter, blur, overlay save), rebuild field value +
     context and call `validate_partial(pointer, value)`.
   - Errors stored per field; summary counts aggregated for status bar.
3. **Form-level validation**: triggered on Save or explicit command; runs full
   validator to ensure `$ref` constraints are checked.
4. **Status surfaces**: Field inline message + status bar bullet
   (`• N errors`) + optional global list in overlay (Ctrl+G).

## 6. State & Module Layout

```
src/
 ├─ schema/
 │   ├─ loader.rs        (serde + schemars)
 │   ├─ resolver.rs      ($ref arena)
 │   ├─ layout.rs        (SectionTree)
 │   └─ metadata.rs
 ├─ domain/
 │   ├─ model.rs         (FormSchema, FieldKind, etc.)
 │   └─ validator.rs     (partial/full validation helpers)
 ├─ form/
 │   ├─ state.rs         (RootState/SectionState/FieldState)
 │   ├─ actions.rs       (enum Command)
 │   └─ reducers.rs      (apply command to state)
 ├─ presentation/
 │   ├─ input.rs         (InputRouter + keymap registry)
 │   ├─ components/
 │   │   ├─ sections.rs
 │   │   ├─ fields.rs
 │   │   ├─ overlay.rs
 │   │   └─ status.rs
 │   └─ theme.rs
 └─ app/
     ├─ controller.rs    (event loop + action bus)
     └─ options.rs
```

Each module should expose pure structs + minimal trait impls; no module > 600
LOC.

### 6.1 Data Structures (Draft)

```rust
// schema/layout.rs
pub struct SectionTree {
    pub roots: Vec<SectionNode>,
}

pub struct SectionNode {
    pub id: SectionId,
    pub title: String,
    pub path: JsonPointer,
    pub description: Option<String>,
    pub ordinal: usize,
    pub children: Vec<SectionNode>,
    pub fields: Vec<FieldNode>,
}

pub struct FieldNode {
    pub id: FieldId,
    pub kind: FieldKind,
    pub pointer: JsonPointer,
    pub required: bool,
    pub metadata: FieldMeta,
}
```

Rendered state mirrors layout but adds UI-specific data:

```rust
pub struct RootState {
    pub id: SectionId,
    pub sections: Vec<SectionState>,
    pub tab_scroll: usize,
}

pub struct SectionState {
    pub node_id: SectionId,
    pub depth: usize,
    pub fields: Vec<FieldState>,
    pub scroll_offset: usize,
}

pub struct FieldState {
    pub node_id: FieldId,
    pub value: FieldValue,
    pub dirty: bool,
    pub error: Option<String>,
    pub cursor: CursorState,
}
```

Reducers operate on these states via immutable commands (no UI coupling).

## 7. Keyboard Map (inspired by nano/helix)

| Action                           | Keys                                      | Notes                                                 |
| -------------------------------- | ----------------------------------------- | ----------------------------------------------------- |
| Move next field                  | `Tab`, `↓`, `Ctrl+F`                      | Helix uses `Ctrl+F` for forward; Tab remains primary. |
| Move prev field                  | `Shift+Tab`, `↑`, `Ctrl+B`.               |                                                       |
| Next section within root         | `Ctrl+Tab`, `Alt+→`, `Ctrl+PageDown`.     |                                                       |
| Prev section within root         | `Ctrl+Shift+Tab`, `Alt+←`, `Ctrl+PageUp`. |                                                       |
| Next root section                | `Ctrl+]`, `Alt+PageDown`.                 |                                                       |
| Prev root section                | `Ctrl+[`, `Alt+PageUp`.                   |                                                       |
| Open selection popup             | `Enter`.                                  |                                                       |
| Confirm selection / overlay save | `Ctrl+S`, `Ctrl+Enter`.                   |                                                       |
| Cancel overlay                   | `Esc` (double Esc discards dirty).        |                                                       |
| Global help                      | `Ctrl+G` (nano-style).                    |                                                       |
| Toggle validation summary        | `Ctrl+/` (helix “command palette” vibe).  |                                                       |

Input routing:

1. `InputRouter` normalizes terminal-specific escapes into semantic `KeyAction`.
2. `Keymap` struct maps `KeyAction` to `Command`, customizable per user config
   later.

### 7.1 KeyAction Examples

```rust
enum KeyAction {
    NextField,
    PrevField,
    NextSection,
    PrevSection,
    NextRoot,
    PrevRoot,
    OpenPopup,
    Save,
    Cancel,
    ToggleErrors,
    ShowHelp,
    JumpTo(String),
    Custom(&'static str),
}
```

`InputRouter` responsibilities:

1. Consume raw `crossterm::Event`.
2. Handle platform quirks (e.g., translate CSI sequences on Windows).
3. Debounce repeated Esc for overlay discard.
4. Forward `KeyAction` to `Keymap`, which resolves to `Command`.

## 8. Sections, Fields & Overlays

### Sections Pane

- Root row: horizontally scrollable tabs.
- Child row: displays tree for active root; nested items show `›` depth markers;
  selected node may be a section or leaf cluster.
- Command palette supports quick jump: `Ctrl+J` opens search, type
  “runtime.http.limits”.

### Fields Pane

- Layout pipeline ensures `name → input → hint → error`.
- Multi-line values show scrollbars; arrays/composites display summary badges
  (count, active variant).

### Overlay Pane

Three overlay templates:

1. **Composite overlay**: left column variant list, right column nested form.
2. **Array overlay**: list of entries + inline editor (reuses field components).
3. **KeyValue overlay**: table-style editing.

Each overlay has independent `FormState`, reusing same reducer/actions so
validation behavior is consistent.

### 8.1 Navigation Diagram

```
┌─────────────┐
│ Root Tabs   │  Ctrl+] / Ctrl+[ switch focus
└────┬────────┘
     │ active root
┌────▼────────┐
│ Section Tree│  Ctrl+Tab cycles siblings; Ctrl+J jump search
└────┬────────┘
     │ selected section
┌────▼────────┐
│ Field List  │  Tab / Shift+Tab, Enter to open popup
└────┬────────┘
     │ array/composite field
┌────▼────────┐
│ Overlay     │  Shares reducers, validates on save/cancel
└─────────────┘
```

## 9. Value & Validation Lifecycle

1. User edits field.
2. FieldState updates internal `FieldValue`.
3. `Reducer` dispatches `Command::FieldChanged(pointer, value)` → updates data
   model + marks dirty.
4. `Validator` runs `validate_partial` on pointer; errors returned as
   `Vec<ValidationError>` mapped to fields.
5. UI refresh highlights field + updates status bar.
6. On Save, call `form_state.try_build_value`, run full validator, emit JSON to
   caller.

## 10. Extensibility & Metadata

Support `x-ui` hints:

- `x-ui-widget`: `slider`, `textarea`, `secret`.
- `x-ui-group`: override section placement/title. Expose `UiRegistry` trait so
  downstream crates can register custom field renderers without forking core.

## 11. Implementation Roadmap (High-level)

1. **Foundations**: carve out new `schema` module (loader, resolver, layout).
   Add unit tests for `$ref`, `oneOf`.
2. **State refactor**: replace flat `FormState` with root→section tree, add
   reducers & command bus.
3. **UI rewrite**: rebuild sections pane + fields pane to consume tree state;
   plug new keymap.
4. **Validation refresh**: implement partial validation + status surfaces.
5. **Overlay alignment**: ensure composite/array/keyvalue overlays reuse
   reducers + validation.
6. **QA**: run sample schemas (current demo + edge cases), verify shortcuts
   across macOS/Linux/Windows terminals.

Deliverable for this phase: updated architecture plus migration plan; follow-up
tickets will implement modules iteratively with `cargo check` enforcing
invariants.

## 12. Detailed Pipeline Walkthrough

1. **Load JSON**
   ```rust
   let raw: Value = serde_json::from_reader(reader)?;
   let root: RootSchema = serde_json::from_value(raw.clone())?;
   ```
2. **Build SchemaArena**
   - BFS through `root.schema`, assign `SchemaId`.
   - Cache resolved `$ref` -> `SchemaId`.
3. **Decorate Metadata**
   - `title = metadata.title.unwrap_or(prettify(name))`.
   - Merge `x-ui` hints (widget, group, order).
4. **Emit SectionTree**
   - Top-level `properties` => `SectionNode`.
   - Recursively add child nodes; preserve `ordinal` from `IndexMap`.
   - Scalar leaves become `FieldNode` with `FieldKind`.
5. **Hydrate FormState**
   - Map each `FieldNode` to `FieldState` (value + cursor).
   - Build `RootState` vector; preselect first focusable field.
6. **Render / Interact**
   - `AppController` polls crossterm events, routes via InputRouter -> Command
     -> Reducer -> FormState.
   - `View` consumes FormState snapshots (no mutation).
7. **Persist / Export**
   - On Save, `FormState::try_build_value()` merges field values into JSON.
   - Validator ensures final object conforms; errors block exit.

## 13. Validation & Error Surfacing Details

- **Partial validation** uses `jsonschema::paths::PathChunk` to target a
  pointer, reducing CPU overhead.\
  Implementation stub:
  ```rust
  fn validate_partial(pointer: &JsonPointer, value: &Value) -> Vec<ValidationError> {
      validator.iter_errors_at(pointer, value).collect()
  }
  ```
- **Error priority**: field-level > section-level > global.
- **UI Cues**:
  - Field label turns red; inline error text appended.
  - Section tree shows badge `(!)` if any descendant field has error.
  - Root tabs show aggregated counts (e.g., `Runtime [2]`).
- **Auto-validate toggle** in options; default ON since spec demands post-edit
  check.

## 14. Shortcut Design Rationale

| Principle       | Application                                                                                       |
| --------------- | ------------------------------------------------------------------------------------------------- |
| Familiarity     | Borrow `Ctrl+G` (nano help), `Ctrl+S` (save), `Esc` (cancel) to reduce onboarding friction.       |
| Redundancy      | Provide Alt/PageDown alternatives for laptops lacking Fn keys.                                    |
| Chaining        | Jump actions (`Ctrl+J`) accept text, enabling helix-style “goto section” without mouse.           |
| Discoverability | Status bar cycles hints relevant to current context; pressing `Ctrl+G` opens cheat-sheet overlay. |

Keymaps stored in `~/.config/schemaui/keymap.toml` later; defaults defined in
code.

## 15. Testing & Tooling Strategy

1. **Unit tests**
   - `schema/layout.rs`: verify SectionTree for sample schema (nested + arrays).
   - `domain/validator.rs`: ensure partial validation isolates pointers.
2. **Golden tests**
   - Render snapshot tests via `ratatui::buffer` to ensure layout stability.
3. **Integration**
   - CLI harness runs `cargo test --all` + `cargo fmt -- --check`.
   - Scenario tests: feed example schema, simulate key events (using `crossterm`
     event queue), assert state transitions.
4. **CI gating**
   - Lint (clippy) + fmt + test.

## 16. Migration Plan & Risks

| Milestone | Deliverable                            | Risk                               | Mitigation                                                 |
| --------- | -------------------------------------- | ---------------------------------- | ---------------------------------------------------------- |
| M1        | Schema module (loader/resolver/layout) | `$ref` cycles                      | Detect loops via `HashSet`, error early.                   |
| M2        | New FormState + reducers               | Complex pointer joins              | Add `JsonPointer` helper + tests.                          |
| M3        | Input router/keymap                    | Terminal chords vary               | Provide fallback combos, add platform-specific unit tests. |
| M4        | UI rewrite (sections/fields)           | Layout regressions                 | Buffer snapshot tests.                                     |
| M5        | Overlay + validation refresh           | Performance of per-edit validation | Cache validator + short-circuit unaffected pointers.       |
| M6        | Shortcut QA across OS                  | OS-specific encoding               | Document known differences; allow config overrides.        |

Rollback strategy: keep old modules under `legacy/` until new pipeline proven;
feature flag `--legacy-ui` toggles old behavior during transition.

## 17. Open Questions

1. Should we expose plugin API for custom validators? (proposal: allow
   user-supplied closures post JSONSchema check). A: do not support currently,
   just use inlay validator (follow json schema standard) for mvp
2. Do we need persistence hooks (auto-save) before exit? (future work). A: do
   not support currently, just return valid result when user sucessfully saved
   like `echo a` , just print a, the next pipe can do anything on the `a` from
   `echo`..
3. Theme customization (colors, fonts) — out of scope for refactor but layout
   should not hard-code colors. A: do not support currently, just hardcode for
   mvp

Answers to these will be revisited once foundational refactor lands.
