# SchemaUI TUI Complex Types – Implementation Specification

## 1. Scope and Goals

This document describes the implementation plan for **complex type support in
the TUI frontend** of `schemaui`, focusing on:

- Correct and clear behaviour for **oneOf / anyOf** and **composite arrays** in
  TUI.
- Fixes for the concrete issues observed when running
  `examples/complex.schema.json` via the TUI:
  - `/b/b1` – ambiguous oneOf variants and unclear labels.
  - `/c/c1/c2/options` – anyOf multi-choice of lists with confusing labels and
    stale summaries.
  - `/d/d1/d2/d3/config/features` – `object[]` array cannot be edited.
  - `/e/e1/e2/e3/e4/deepItems` – multi-choice[] entry where variant selection
    does not affect the value.
- Aligning TUI behaviour with the **Web UI**, which is already largely correct
  for the same schema.
- Removing or replacing legacy behaviours (e.g. plain JSON array editing for
  `object[]`) instead of layering new logic on top.

This spec is written against the current code in the `schemaui` repository and
references concrete files and functions that will be changed.

## 2. Current Architecture Overview (Complex Types)

### 2.1 High-level pipeline

The shared pipeline for both TUI and Web frontends is:

```text
JSON Schema → core::schema_core / resolver → ui_ast::build_ui_ast
           → tui::model::form_schema_from_ui_ast
           → tui::state::FormState
           → tui::view::render_fields (ratatui)
```

ASCII diagram:

```text
+------------------+        +------------------------+
|  JSON Schema     |        |  core::schema_core    |
|  (examples/*.json) -----> |  loader / resolver    |
+------------------+        +------------------------+
                                       |
                                       v
                               +-----------------+
                               | ui_ast::builder |
                               | build_ui_ast    |
                               +-----------------+
                                       |
                                       v
                           +--------------------------+
                           | tui::model::form_schema |
                           | form_schema_from_ui_ast |
                           +--------------------------+
                                       |
                                       v
                              +------------------+
                              | tui::state       |
                              | FormState,       |
                              | FieldState, ...  |
                              +------------------+
                                       |
                                       v
                              +------------------+
                              | tui::view        |
                              | render_fields    |
                              +------------------+
```

The complex-type specific parts live in:

- **UiAst & builder**
  - `src/ui_ast/types.rs`
    - `UiNodeKind::{Array, Composite, Object, Field}`
    - `UiVariant { id, title, description, is_object, node, schema }`
  - `src/ui_ast/builder.rs`
    - `build_ui_ast(raw: &Value) -> Result<UiAst>`
    - `visit_schema`, `visit_kind`
    - `build_composite_node`, `build_composite_kind`, `build_variants`
    - `default_variant_title` – _current variant label heuristics_.

### 2.2 TUI model – FormSchema and FieldKind

The TUI has its own legacy model that adapts `UiAst` into a tree that the state
layer can consume:

- **File**: `src/tui/model/form_schema.rs`
  - `FormSchema { roots: Vec<RootSection> }`
  - `RootSection { id, title, description, sections: Vec<FormSection> }`
  - `FormSection { id, title, description, path, fields, children }`
  - `FieldSchema { name, path, pointer, title, description, kind, required, default, metadata }`
  - `FieldKind` enum:
    - `String | Integer | Number | Boolean`
    - `Enum(Vec<String>)`
    - `Array(Box<FieldKind>)`
    - `Json` (generic object)
    - `Composite(Box<CompositeField>)`
    - `KeyValue(Box<KeyValueField>)`
  - `CompositeField { mode: CompositeMode, variants: Vec<CompositeVariant> }`
  - `CompositeVariant { id, title, description, schema, is_object }`

- **File**: `src/tui/model/ui_ast_adapter.rs`
  - `form_schema_from_ui_ast(ast: &UiAst) -> FormSchema`
  - Maps `UiNodeKind` to `FieldKind`:
    - `Field` → scalar or `Enum`.
    - `Array{ item }` →
      `FieldKind::Array(Box::new(field_kind_from_node_kind(item)))`.
    - `Composite{ mode, variants }` →
      `FieldKind::Composite(Box<CompositeField>)`.
    - `Object` → `FieldKind::Json` (used for generic objects and `object[]`).

This layer is **frontend-agnostic**; it knows nothing about Ratatui or
keybindings.

### 2.3 TUI state – FormState, FieldState and CompositeState

The TUI state layer turns `FormSchema` into live, editable state:

- **File**: `src/tui/state/form_state.rs`
  - `FormState` owns all `SectionState` and `FieldState` instances.
  - Responsible for:
    - Focus management between roots/sections/fields.
    - Seeding values: `seed_from_value(value: &Value)`.
    - Building final value:
      `try_build_value() -> Result<Value, FieldCoercionError>`.
    - Error tracking: `set_error`, `clear_error`, `error_count`, `is_dirty`.

- **File**: `src/tui/state/field/state/mod.rs`
  - `FieldState { schema: FieldSchema, component: Box<dyn FieldComponent>, dirty, error }`.
  - Uses `FieldComponent` implementors for kind-specific behaviour.
  - Split into submodules:
    - `builder.rs` – constructs `FieldComponent` from `FieldSchema`.
    - `input.rs` – handles key events and simple setters.
    - `lists.rs` – handles composite lists, editors, multi-select and popups.
    - `value_ops.rs` – seed, display and current value.

- **File**: `src/tui/state/composite.rs`
  - `CompositeState` – manages oneOf/anyOf **single-value** composites.
    - Holds `variants: Vec<CompositeVariantState>`.
    - Tracks `mode: CompositeMode` and `pointer` (JSON pointer).
    - Provides:
      - `summary()` – returns a label like `"Variant: #1 Simple"` or
        `"Variants: #1 Simple, #2 Numeric"`.
      - `seed_from_value(value)` – picks best-matching variant and seeds its
        internal `FormState`.
      - `apply_single(index)` / `apply_multi(flags)` – update active variant(s).
      - `build_value(required)` – build the effective JSON value from active
        variants.
  - `CompositeVariantState` – wraps one variant schema and its lazy `FormState`.
    - `overlay_schema()` – possibly wraps non-object schemas into
      `{ "__value": ... }`.
    - `snapshot()` – produces
      `CompositeVariantSummary { title, description, lines }` used for inline
      summaries.
    - `matches_value(obj: &Map<String, Value>)` – uses `const` and `enum`
      properties to identify matching variants.

- **File**: `src/tui/state/composite/composite_list.rs`
  - `CompositeListState` – manages arrays of composites (`Array<Composite>`),
    e.g. `/b/b1`, `deepItems`.
    - Tracks `entries: Vec<CompositeListEntry>` with individual
      `CompositeState`.
    - Provides operations:
      - `add_entry(variant_index: Option<usize>)`, `remove_selected()`,
        `move_selected(delta)`.
      - `selected_label()` and `summaries()` for inline display.
      - `selected_entry_selector()` and `popup()` for variant selection.
      - `build_value() -> Result<Option<Value>, FieldCoercionError>`.

This state layer is **where most complex-type semantics live**; the TUI view
simply asks for summaries and panels.

### 2.4 TUI view – field rendering and summaries

The Ratatui view code lives in:

- **File**: `src/tui/view/components/fields.rs`
  - `render_fields(frame, area, form_state, enable_cursor)` – entry point.
  - For each `FieldState`, calls
    `build_field_render(field, is_selected, max_width)`.
  - `build_field_render` composes:
    - `info_line(field, is_selected)` – label, `*`, `·dirty`, `·invalid`.
    - `value_panel_lines(field, is_selected, max_width)` – bordered box with
      `field.display_value()`.
    - `meta_lines(field, is_selected, max_width)` –
      `type: ... | constraints: ...`.
    - When selected:
      - `composite_selector_lines(field)` – oneOf/anyOf selector summary.
      - `composite_summary_lines(field)` – active variant summaries.
      - `repeatable_summary_lines(field)` – composite list entries summary.

Key helpers for complex types:

- `field_type_label(kind: &FieldKind) -> String`
  - `FieldKind::Composite(OneOf)` → `"choice"`.
  - `FieldKind::Composite(AnyOf)` → `"multi-choice"`.
  - `FieldKind::Array(Composite(OneOf))` → `"choice[]"` or `<single-title>[]`.
  - `FieldKind::Array(Composite(AnyOf))` → `"multi-choice[]"`.
  - `FieldKind::Array(Json)` → `"object[]"`.

- `composite_selector_lines(field: &FieldState)`
  - Uses `field.composite_selector_view()` (from
    `FieldState::composite_selector_view`).
  - Renders labels like:

    ```text
    AnyOf (value satisfies at least one)
      [ ] #1 List
      [ ] #2 List
    ```

- `repeatable_summary_lines(field: &FieldState)`
  - Uses `field.composite_list_panel()` (ultimately from
    `CompositeListState::summaries`).
  - For `/b/b1` / `deepItems`, shows entries like:

    ```text
    Entries:
      » #1 Variant: #1 Simple item
    ```

### 2.5 Popups, keybindings and variant selection

Popups and keybindings act as the glue between user input and `FieldState`:

- **File**: `src/tui/app/keymap.rs`
  - Defines `KeymapContext`, `RawAction`, and `KeyAction` enums.
  - Binds keys (from `keymap/default.keymap.json`) to actions, e.g.:
    - `TogglePopup` – open enum / composite / multi-select popup.
    - `EditComposite` – open composite / composite list / array editor overlay.
    - `ListAddEntry`, `ListRemoveEntry`, `ListMove`, `ListSelect` – manipulate
      repeatable collections.

- **File**: `src/tui/app/input.rs`
  - Maps `KeyAction` to `CommandDispatch` (App vs Form commands).
  - `AppCommand::TogglePopup` → `App::try_open_popup(...)`.
  - `AppCommand::EditComposite` → `App::try_open_composite_editor()`.

- **File**: `src/tui/app/popup.rs`
  - `PopupState::from_field(field: &FieldState) -> Option<Self>`:
    - If `field.multi_options()` → builds multi-select popup.
    - Else if `field.bool_value()` → boolean popup.
    - Else if `field.enum_state()` → enum popup.
    - Else if `field.composite_popup()` → composite / composite-list popup.

- **File**: `src/tui/app/runtime/overlay/app/popup_ops.rs`
  - `apply_selection_to_field(field: &mut FieldState, selection: usize, multi: Option<Vec<bool>>)`:

    ```rust
    if let Some(flags) = multi {
        match &field.schema.kind {
            FieldKind::Composite(_) => {
                field.apply_composite_selection(selection, Some(flags));
            }
            FieldKind::Array(inner) if matches!(inner.as_ref(), FieldKind::Enum(_)) => {
                field.set_multi_selection(&flags);
            }
            _ => {}
        }
        return;
    }

    match &field.schema.kind {
        FieldKind::Composite(_) => field.apply_composite_selection(selection, None),
        FieldKind::Boolean => field.set_bool(selection == 0),
        FieldKind::Enum(_) => field.set_enum_selected(selection),
        _ => {}
    }
    ```

  - **Important gap**: there is currently **no branch** for
    `FieldKind::Array(inner)` where `inner` is `FieldKind::Composite(_)`. That
    means:
    - Popups for `Array<Composite>` (e.g. `deepItems`) can render correctly.
    - But toggling variants in the popup **does not change** the underlying
      `CompositeListState`.

## 3. Problem Statements (as observed in TUI)

This section recaps the user-observed behaviour for
`examples/complex.schema.json`, mapped to the current implementation.

### 3.1 `/b/b1` – ambiguous oneOf variants and confusing overlay

**Observed behaviour** (TUI):

- Focusing `/b/b1` and pressing Enter shows a popup with two options:

  ```text
  » object with id
    object with id
  ```

- Choosing the first, then pressing `Ctrl+E` opens an overlay where:

  - `Entries: #1 Variant: #1 object with id`
  - Fields: `Enabled`, `Id`, `Kind`, `Label`.

- Choosing the second, then pressing `Ctrl+E` opens an overlay that _looks the
  same_.

**Likely causes** (code):

- Variant titles are generated in `ui_ast::builder::default_variant_title`:
  - For object schemas without a clear `type` or `$ref`, the fallback path names
    them like `"object with id"`.
  - Both `simpleItem` and `numericItem` variants contain an `id` field, so they
    both get the same title.
- TUI composite logic (`CompositeState` and `CompositeVariantState`) correctly
  maintains **separate forms per variant**, but the overlay UI:
  - Uses the generic title (`object with id`) for both.
  - Does not surface the structural differences (`label`/`enabled` vs `values`)
    clearly.

**Impact**:

- Users cannot easily distinguish the `simple` vs `numeric` variants when
  selecting or editing entries in `/b/b1`.
- Combined with weak labels, this creates the impression that both variants are
  identical, even though the underlying schemas differ.

### 3.2 `/c/c1/c2/options` – anyOf multi-choice of lists with stale summaries

**Observed behaviour** (TUI):

- Inline meta for `options` shows:

  ```text
  type: multi-choice
      AnyOf (value satisfies at least one)
        [ ] #1 List
        [ ] #2 List
      Press Enter to toggle • Ctrl+E to open editor
      Active: <none selected>
  ```

- The popup labels two choices as `List`, and width truncation can make them
  appear as `Li`.
- After selecting both variants and editing each list to add elements:
  - Overlay 1 (per-variant editor) shows correct list contents.
  - Returning to the main view, the inline summary under `Active variants:`
    still shows:

    ```text
    • #1 List
       Section: General
         • List (__value) = Array: empty (Ctrl+Left/Right select, Ctrl+E edit)
    • #2 List
       Section: General
         • List (__value) = Array: empty (Ctrl+Left/Right select, Ctrl+E edit)
    ```

**Likely causes** (code):

- Variant titles again come from `default_variant_title` and treat both
  `array<string>` and `array<integer>` as a generic `List`.
- `CompositeVariantState.snapshot()` builds summaries from the internal
  `FormState` for each variant:
  - For wrapped non-object variants, the array-of-scalar field is exposed as
    `List (__value)`.
  - The TUI shows `Array: empty` when the **underlying scalar array state** is
    still empty.
- There is a mismatch between:
  - The overlay/editor applied to the list field, and
  - The `ScalarArrayState` that `snapshot()` reads from:
    - Either the editor result is **not** being written back to the same
      `FieldState`.
    - Or that `FieldState` is later re-seeded/overwritten with defaults.

**Impact**:

- Users correctly edit list contents inside overlays, but the main TUI never
  reflects that work.
- The summary `Array: empty` is misleading and makes it look like data was lost.

### 3.3 `/d/d1/d2/d3/config/features` – `object[]` cannot be edited

**Observed behaviour** (TUI):

- Inline shows a small box and:

  ```text
  Features
    ┌────┐
    │ [] │
    └────┘
      type: object[]
  ```

- Pressing Enter or `Ctrl+E` has no effect – there is no way to add or edit
  items.

**Likely causes** (code):

- The schema describes `features` as an **array of generic objects** without
  `oneOf`/`anyOf`.
- In `ui_ast_adapter::field_kind_from_node_kind`:
  - `UiNodeKind::Object` maps to `FieldKind::Json`.
  - `UiNodeKind::Array { item: Object }` maps to
    `FieldKind::Array(Box::new(FieldKind::Json))`.
- In `FieldState::from_schema_with_palette` (`state/field/state/builder.rs`):

  ```rust
  FieldKind::Array(inner) => match inner.as_ref() {
      FieldKind::Enum(options) => MultiSelectComponent,
      FieldKind::Composite(meta) => CompositeListComponent,
      FieldKind::String | Integer | Number | Boolean => ScalarArrayComponent,
      _ => ArrayBufferComponent,
  }
  ```

- Therefore, `object[]` is handled by `ArrayBufferComponent`:
  - A plain text buffer for JSON arrays.
  - No list entries, no per-item editors, no `CompositeListState`.

**Impact**:

- From a user perspective, `features` appears as a dead, non-interactive field.
- This is also inconsistent with Web UI expectations, where such arrays are
  typically rendered as editable lists of objects.

### 3.4 `/e/e1/e2/e3/e4/deepItems` – multi-choice[] variant selection does not apply

**Observed behaviour** (TUI):

- Inline for `deepItems` shows:

  ```text
  Deep Items (deepItems)  ·dirty  ·invalid
    type: multi-choice[]
    ┌────────────────────────────────────────────────────────┐
    │ List[1] • #1 Variants: #1 Object (Ctrl+Left/Right ...) │
    └────────────────────────────────────────────────────────┘
      type: multi-choice[]
      Entries:
      » #1 Variants: #1 Object
      Error:
        [{"active":false,"priority":0,"url":""}] has less than 2 items
  ```

- Enter opens a popup labelled `Deep Items` with options roughly like:

  ```text
  Deep Items ┐
  │  [x] Object │
  │» [ ] Text   │
  │  [ ] Integer│
  ```

- Toggling `Text` / `Integer` and confirming has **no effect** on the inline
  view:
  - Still shows `#1 Variants: #1 Object`.
  - Validation errors remain unchanged.

**Root cause** (code):

- The field kind for `deepItems` is
  `FieldKind::Array(Box::new(FieldKind::Composite(AnyOf ...)))`.
- Popups for `Array<Composite>` are created by
  `CompositeListComponent::composite_popup()`/`CompositeListState::popup()`.
- However, `apply_selection_to_field` in `popup_ops.rs` ignores this case:
  - Only `FieldKind::Composite` and `FieldKind::Array(Enum)` are wired to apply
    multi flags.
  - `Array(Composite)` never sees the new flags → `CompositeListState`’s active
    variants remain unchanged.

**Impact**:

- Users cannot change the active variant(s) for entries in `deepItems` via the
  popup.
- Error messages referring to specific variants and constraints never update,
  even when a different variant is selected in the UI.

## 4. Target Design (no legacy fallbacks)

We want a **single, coherent design** for complex types in TUI, without legacy
fallbacks such as:

- Treating `object[]` as a raw JSON string buffer.
- Having separate, partially overlapping logic paths for composite lists vs
  composites.

The target design respects the existing pipeline and public behaviour but
simplifies the internal rules:

1. **Variant naming** is derived from schema semantics (const fields, item
   types, references), not from ad-hoc fallbacks.
2. **AnyOf/OneOf variant selection** always flows through the same APIs:
   - `FieldState::composite_selector_view()` → `PopupState` →
     `apply_selection_to_field` → `CompositeState`/`CompositeListState`.
3. **Composite arrays** (`Array<Composite>`) are always edited via
   `CompositeListState` and associated components, including:
   - `/b/b1` – list of object variants.
   - `/e/.../deepItems` – list of anyOf entries.
   - `/d/.../features` – list of plain objects (treated as a single-variant
     composite).
4. **Inline summaries** and overlay views are sourced from a **single state
   owner**:
   - `CompositeVariantState::snapshot()` reads the same `FormState` that overlay
     editing mutates.
   - No hidden re-seeding or shadow state.

### 4.1 ASCII overview – desired data flow for composite arrays

```text
JSON Schema                             TUI Runtime
-----------                             ----------

Array<Composite>                      +-------------------+
(anyOf/oneOf item schemas)  --->      | UiAst (Composite) |
                                      +-------------------+
                                                |
                                                v
                                      +------------------------+
                                      | FieldKind::Array(      |
                                      |   Composite{variants}  |
                                      +------------------------+
                                                |
                                                v
                                      +------------------------+
                                      | CompositeListState     |
                                      |  entries: Vec<         |
                                      |    CompositeState      |
                                      |  >                     |
                                      +------------------------+
                                                |
       Popup (AnyOf/OneOf)                      v
  +---------------------------+        +------------------------+
  | PopupState (options,      |  --->  | apply_selection_to_    |
  |  toggles, selection)      |        | field ->               |
  +---------------------------+        |  CompositeListState    |
                                       +------------------------+
                                                |
                                                v
                                      +------------------------+
                                      | CompositeListState::   |
                                      |  build_value()         |
                                      +------------------------+
                                                |
                                                v
                                      +------------------------+
                                      | FormState::try_build_ |
                                      |  value()              |
                                      +------------------------+
```

## 5. Implementation Plan (per issue)

This section describes, for each observed issue, the concrete code changes to
implement, including pseudocode and references.

### 5.1 Improve variant titles (fix `/b/b1` and clarify list variants in `options`)

**Goal:**

- Make oneOf/anyOf variants clearly distinguishable in TUI by improving the
  heuristics in `ui_ast::builder::default_variant_title`.
- Use schema semantics (const fields, item types) instead of generic fallbacks
  like `"object with id"` or `"List"`.

**Files:**

- `src/ui_ast/builder.rs`
  - `default_variant_title(index: usize, schema: &SchemaObject) -> String`
  - helpers: `instance_type`, `get_const_value`, `humanize_identifier`.

**Changes:**

1. **Recognise `kind` const fields for object variants.**

   Pseudocode:

   ```rust
   fn default_variant_title(index: usize, schema: &SchemaObject) -> String {
       // 1) If this is a $ref, keep existing behaviour
       if let Some(reference) = schema.reference.as_ref() {
           if let Some(name) = reference.split('/').next_back() {
               return humanize_identifier(name);
           }
       }

       // 2) Look for a discriminant const field
       if let Some(obj) = schema.object.as_ref() {
           // Prefer `type` const if present (existing behaviour)
           if let Some(type_title) = const_string_field(schema, "type") {
               return type_title;
           }

           // NEW: also look for `kind` const as a discriminant
           if let Some(kind_title) = const_string_field(schema, "kind") {
               // e.g. "simple" -> "Simple", "numeric" -> "Numeric"
               return humanize_identifier(&kind_title);
           }

           // Fallback: keep existing `object with id` / `object with name` logic
           if obj.properties.contains_key("id") {
               return "object with id".to_string();
           }
           if obj.properties.contains_key("name") {
               return "object with name".to_string();
           }
       }

       // 3) Keep existing array / scalar fallback logic
       // ... (unchanged for now)
   }
   ```

   - Helper `const_string_field(schema, field_name)` traverses
     `object.properties[field_name].const` and returns it as `String` if
     present.
   - This change directly affects `/b/b1` variants, turning them into titles
     like `"Simple"` / `"Numeric"` instead of two copies of `"object with id"`.

2. **Distinguish array-of-scalar variants (e.g. `List<string>` vs
   `List<integer>`).**

   Enhance the **array** part of `default_variant_title`:

   ```rust
   // For arrays, describe what kind of array
   if let Some(array) = schema.array.as_ref() && let Some(items) = &array.items {
       // existing ref-name logic ...

       // NEW: if items is a simple scalar type, name as List<type>
       let item_type = detect_scalar_item_type(items)?; // returns Option<&'static str>
       if let Some(kind) = item_type {
           return format!("List<{}>", kind); // e.g. "List<string>", "List<integer>"
       }
   }
   ```

   - `detect_scalar_item_type` can re-use `detect_scalar` logic, but constrained
     to the item schema.
   - This change makes `/c/c1/c2/options` variants show up as `List<string>` and
     `List<integer>` instead of two identical `List` entries.

**Result:**

- TUI popups and overlays will show precise variant names:
  - `/b/b1` popup: `"Simple"` vs `"Numeric"`.
  - `options` popup: `"List<string>"` vs `"List<integer>"`.
- These titles are wired automatically into:
  - `CompositeState::summary()` → `"Variant: #1 Simple"`.
  - `CompositeListState::summaries()` → `"#1 Variant: #1 Simple"`.
  - TUI view’s `composite_selector_lines` and `repeatable_summary_lines`.

### 5.2 Apply AnyOf / OneOf selection to composite arrays (fix `deepItems`)

**Goal:**

- Ensure that toggling variants in the popup for `Array<Composite>` fields (such
  as `/e/e1/e2/e3/e4/deepItems`) actually updates the underlying
  `CompositeListState`.

**Files:**

- `src/tui/app/runtime/overlay/app/popup_ops.rs`
  - `apply_selection_to_field(field: &mut FieldState, selection: usize, multi: Option<Vec<bool>>)`.

**Current behaviour (excerpt):**

```rust
if let Some(flags) = multi {
    match &field.schema.kind {
        FieldKind::Composite(_) => {
            field.apply_composite_selection(selection, Some(flags));
        }
        FieldKind::Array(inner) if matches!(inner.as_ref(), FieldKind::Enum(_)) => {
            field.set_multi_selection(&flags);
        }
        _ => {}
    }
    return;
}

match &field.schema.kind {
    FieldKind::Composite(_) => {
        field.apply_composite_selection(selection, None);
    }
    FieldKind::Boolean => field.set_bool(selection == 0),
    FieldKind::Enum(_) => field.set_enum_selected(selection),
    _ => {}
}
```

`FieldKind::Array(Composite(_))` is missing here.

**Planned change:**

- Extend both branches to treat `Array<Composite>` like a composite list,
  delegating to `FieldState::apply_composite_selection` which internally routes
  to `CompositeListState::apply_selection`:

Pseudocode:

```rust
if let Some(flags) = multi {
    match &field.schema.kind {
        FieldKind::Composite(_) => {
            field.apply_composite_selection(selection, Some(flags));
        }
        FieldKind::Array(inner) if matches!(inner.as_ref(), FieldKind::Composite(_)) => {
            // NEW: array of composite (CompositeListComponent)
            field.apply_composite_selection(selection, Some(flags));
        }
        FieldKind::Array(inner) if matches!(inner.as_ref(), FieldKind::Enum(_)) => {
            field.set_multi_selection(&flags);
        }
        _ => {}
    }
    return;
}

match &field.schema.kind {
    FieldKind::Composite(_) => {
        field.apply_composite_selection(selection, None);
    }
    FieldKind::Array(inner) if matches!(inner.as_ref(), FieldKind::Composite(_)) => {
        // NEW: single-choice variant change for array entries
        field.apply_composite_selection(selection, None);
    }
    FieldKind::Boolean => field.set_bool(selection == 0),
    FieldKind::Enum(_) => field.set_enum_selected(selection),
    _ => {}
}
```

**Effect on `deepItems`:**

- Popups created by `CompositeListState::popup()` for `deepItems` now:
  - Send their `flags` back to `CompositeListState::apply_selection` via
    `FieldState::apply_composite_selection`.
  - Update `CompositeState` for each entry, which in turn affects:
    - `CompositeListState::build_value()` → actual JSON value.
    - `CompositeListState::summaries()` → `repeatable_summary_lines`.
- Validation errors attached to `/e/e1/e2/e3/e4/deepItems` will now reflect the
  **active variant configuration**, rather than always assuming the `Object`
  variant.

### 5.3 Promote `object[]` to a composite list (fix `features`)

**Goal:**

- Replace legacy handling of `object[]` as a raw JSON buffer with proper list
  editing via `CompositeListState`.
- This keeps the interaction model consistent with other composite arrays while
  reusing existing UI machinery.

**Files:**

- `src/ui_ast/builder.rs`
  - `visit_kind(resolver, schema) -> Result<UiNodeKind>`.
  - `resolve_array_items(resolver, array)`.

**Current behaviour (simplified):**

```rust
if is_array_schema(schema) {
    let array = schema.array.as_ref().context("...")?;
    let items_schema = resolve_array_items(resolver, array)?;
    let item_node = visit_kind(resolver, &items_schema)?;
    return Ok(UiNodeKind::Array {
        item: Box::new(item_node),
        min_items: array.min_items.map(...),
        max_items: array.max_items.map(...),
    });
}
```

For `features` (array of plain objects without oneOf/anyOf), this yields:

- `UiNodeKind::Array { item: UiNodeKind::Object { .. } }`
- → `FieldKind::Array(Box::new(FieldKind::Json))` in
  `tui::model::form_schema_from_ui_ast`.

**Planned change:**

- When we see an **array of object** whose item schema does _not_ itself have
  `oneOf`/`anyOf`, wrap it as a **single-variant composite** instead of a plain
  object:

Pseudocode:

```rust
fn visit_kind(resolver: &SchemaResolver<'_>, schema: &SchemaObject) -> Result<UiNodeKind> {
    if is_array_schema(schema) {
        let array = schema.array.as_ref().context("...")?;
        let items_schema = resolve_array_items(resolver, array)?;

        // NEW: detect "array of plain object" without oneOf/anyOf
        if is_object_schema(&items_schema)
            && items_schema
                .subschemas
                .as_ref()
                .map_or(true, |subs| subs.one_of.is_none() && subs.any_of.is_none())
        {
            // Build a single-variant composite for the item type
            let single = Schema::Object(items_schema.clone());
            let composite_kind = build_composite_kind(
                resolver,
                &[single],
                CompositeMode::OneOf,
            )?;

            return Ok(UiNodeKind::Array {
                item: Box::new(composite_kind),
                min_items: array.min_items.map(|v| v as u64),
                max_items: array.max_items.map(|v| v as u64),
            });
        }

        // Existing behaviour for other array shapes
        let item_node = visit_kind(resolver, &items_schema)?;
        return Ok(UiNodeKind::Array {
            item: Box::new(item_node),
            min_items: array.min_items.map(|v| v as u64),
            max_items: array.max_items.map(|v| v as u64),
        });
    }

    // ... rest unchanged ...
}
```

**Effect on TUI:**

- `features: object[]` becomes:
  - `UiNodeKind::Array { item: Composite{mode: OneOf, variants: [FeatureObject] } }`.
  - `FieldKind::Array(Box::new(FieldKind::Composite(..)))`.
  - Handled by `CompositeListComponent` + `CompositeListState`.
- Users can now:
  - Use list keybindings (`Ctrl+N`, `Ctrl+D`, etc.) to add/remove feature
    entries.
  - Press `Ctrl+E` to open an overlay editor for each feature object (reusing
    the existing composite list editor).
- The old `ArrayBufferComponent` is still used for truly generic arrays (e.g.
  `array<json>` where we _want_ raw JSON editing), but not for plain `object[]`
  where a form-based editor is more ergonomic.

### 5.4 Ensure summaries track edited list contents (fix `options` stale `Array: empty`)

**Goal:**

- Make sure that after editing list contents inside composite overlays (e.g.
  `/c/c1/c2/options`), the inline `Active variants` summaries reflect the
  updated arrays instead of always showing `Array: empty`.

**Files to inspect/change:**

- `src/tui/state/array.rs`
  - `ScalarArrayState::{open_selected_editor, apply_editor_session}`.
- `src/tui/state/field/state/lists.rs`
  - `close_scalar_array_editor`.
- `src/tui/state/composite.rs`
  - `CompositeVariantState::snapshot`.
- `src/tui/app/runtime/overlay/state.rs` / `overlay_legacy.rs`
  - Overlay close paths that call `close_scalar_array_editor` or
    `close_composite_editor`.

**Current intended flow:**

```text
Overlay (Array entry editor)
    ↓ closes with session
ScalarArrayState::apply_editor_session(entry_index, session)
    ↓
ScalarArrayState.entries[entry_index] updated
    ↓
CompositeVariantState::snapshot() iterates fields in form_state
    ↓
field.display_value() for list field → "Array[n] • ..."
    ↓
CompositeVariantSummary.lines used in composite_summary_lines()
```

If the inline summary still shows `Array: empty`, it means one of the following
is happening:

1. The overlay close path is **not** calling `close_scalar_array_editor` /
   `apply_editor_session` for the same `FieldState` that `snapshot()` reads.
2. A later operation (e.g. re-seeding from default) resets the scalar array
   state after the editor closes.

**Planned checks and adjustments:**

1. **Verify overlay close pipeline** for scalar arrays inside composites:

   - In `overlay/state.rs` and `overlay/app/list_ops.rs`, confirm that when a
     scalar array field is edited inside a composite overlay, the close handler
     calls:

     ```rust
     field.close_scalar_array_editor(entry_index, &session, mark_dirty);
     ```

   - Ensure `mark_dirty = true` when edits are saved (so that TUI refreshes
     summaries and marks the field as dirty).

2. **Confirm `CompositeVariantState::snapshot` reads the live `FormState`:**

   - `snapshot(pointer)` obtains `FormState` via `borrow_form(pointer)` and
     iterates sections/fields.
   - No re-seeding should occur between overlay close and snapshot.

3. **Add targeted tests (if feasible):**

   - Unit or integration test that:
     - Seeds a `CompositeState` with a variant containing a scalar array field.
     - Simulates `ScalarArrayState.apply_editor_session` changing an entry.
     - Asserts that `CompositeVariantState::snapshot().lines` contains
       `Array(1)` or the equivalent non-empty summary.

4. **If necessary, avoid unnecessary re-seeding:**

   - Audit any calls to `seed_from_value` on the variant’s `FormState` that
     might run **after** an edit has been applied.
   - Ensure `seed_from_value` is only called when:
     - Initialising from an external value, or
     - Switching variants, not after saving edits inside the same variant.

Given that this is more about **correct wiring** than new behaviour, the spec
keeps this part at the level of checks + guidelines rather than prescribing a
large new API.

## 6. Testing and Validation Plan

After implementing the above changes, we will validate behaviour both manually
and (where possible) via automated tests.

### 6.1 Manual TUI regression (complex.schema.json)

From the repository root:

```bash
just build
./target/debug/schemaui tui -s examples/complex.schema.json
```

(If there is no direct `tui` subcommand yet, run the existing TUI entrypoint as
per `schemaui-cli` docs.)

Test paths:

1. **`/b/b1` oneOf list**
   - Focus `/b/b1`, open variant selector popup:
     - Expect variant labels like `Simple` / `Numeric` instead of two
       `object with id`.
   - Add multiple entries, switch variants with popup + `Ctrl+E`:
     - Verify each entry’s overlay shows fields appropriate to its variant.

2. **`/c/c1/c2/options` anyOf multi-choice of lists**
   - Focus `options` and inspect inline meta:
     - Expect `List<string>` / `List<integer>` labels.
   - Use popup to activate both variants; use overlays to add elements to each
     list.
   - Confirm `Active variants` summary now shows non-empty arrays (e.g.
     `Array(1)`), not `Array: empty`.

3. **`/d/d1/d2/d3/config/features` object[]**
   - Focus `features`:
     - Expect it to be treated as a composite list (`List[0] • ...`), not
       `object[]` + inert box.
   - Use list controls to add/remove entries and edit each feature in an
     overlay.

4. **`/e/e1/e2/e3/e4/deepItems` multi-choice[]**
   - Focus `deepItems`, open popup:
     - Toggle between `Object`, `Text`, and `Integer` variants.
   - Confirm inline summary and validation errors update according to the chosen
     variants.

### 6.2 Web UI parity check

Run the Web UI for the same schema:

```bash
just build
./target/debug/schemaui web -s examples/complex.schema.json --port 5175 -o -
```

Then, with Playwright (or the provided @playwright MCP):

- Navigate to `http://localhost:5175`.
- Repeat the same paths (`/b/b1`, `options`, `features`, `deepItems`).
- Confirm that TUI and Web behaviours are consistent in terms of:
  - Variant defaults and discriminants (`kind: "simple" | "numeric"`).
  - AnyOf/OneOf selection.
  - Array editing behaviour and summaries.

### 6.3 Automated tests (where practical)

- Add focused unit tests around:
  - `default_variant_title` for:
    - Object variants with `kind` const.
    - Array variants with scalar element types.
  - `apply_selection_to_field` for `FieldKind::Array(Composite(_))`.
  - `CompositeVariantState::snapshot` reflecting changes from
    `ScalarArrayState::apply_editor_session`.

- If integration tests exist for TUI state (similar to
  `overlay_stack_tests.rs`), extend them to cover:
  - Adding entries to `CompositeListState` where variants and lists are edited.
  - Applying multi-choice flags to composite arrays.

---

This specification should now be sufficient for another engineer to:

- Understand the current complex-type handling in the TUI.
- Implement the proposed refactors without keeping legacy design paths.
- Validate the new behaviour both in TUI and Web frontends.
