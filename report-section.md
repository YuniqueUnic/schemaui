# Section & Field Redesign – Technical Plan

## 1. Current Findings
After reviewing the parser (`src/domain/parser.rs`), form state (`src/form/state.rs`, `src/form/section.rs`), and presentation layer (`src/presentation/components.rs`), the following structural problems are confirmed:

1. **Flattened sections** – every nested object that satisfies `descend` creates an entry in the global `section_meta` map. As a result, internal nodes like `runtime.http` or `runtime.limits` appear in the top-level Sections tab, which breaks the mental model of “root module” vs “sub-panel”.
2. **Field ordering drift** – although `schemars::Map` preserves insertion order, we currently append fields into the destination `slots[section_id]` without stabilising by the original `properties` order when multiple recursion levels collapse into the same section. Additionally, we append `additionalProperties` pseudo-fields after the loop, which is why “Metadata” appears as a phantom field at the end of the metadata section.
3. **Document Store overlay failure** – composite variants are serialised as isolated JSON documents (see `CompositeVariantState::snapshot`/`take_editor_session`). These documents no longer contain the root-level `definitions`, so `$ref: "#/definitions/endpoint"` inside the variant becomes unreachable and `detect_kind` returns `unsupported schema for field 'endpoints'`.
4. **UI ergonomics** – the Sections tab renders a single flat row and reuses the same keyboard shortcuts for every layer. The product spec now requires:
   - Root sections (General, Metadata, Runtime, …) shown in one row.
   - Nested sections for the currently focused root shown in a second row.
   - `Ctrl+Tab` / `Ctrl+Shift+Tab` keep cycling within the active root’s subsections, while `Ctrl+]` / `Ctrl+[ ` move across root sections.
   - Field layouts (main & overlay) must always follow `name → content → type|description → error`, regardless of the field kind.

## 2. Root Causes
1. **Parser data shape** – `FormSchema` only stores a flat `Vec<FormSection>`. We lose ancestry information during parsing and can’t rebuild a hierarchy later.
2. **Section ids reuse** – `section_info_for_field` and `section_info_for_object` reuse the same `id` (the path segment) for both containers and scalar fields. When an object contains `additionalProperties`, the container path is reinserted as a field, yielding the phantom “Metadata” row.
3. **Composite variant context** – `parse_form_schema` is invoked with the raw variant schema only. `$ref` resolution depends on the original root (with `definitions`), so any reference inside a composite variant fails after extraction.
4. **Rendering code** – `build_field_render` directly mixes content, metadata, and error blocks. Overlay and main form share the same renderer, therefore any ordering bug (like errors being after `type|description`) affects both contexts.

## 3. Proposed Architecture
### 3.1 Data Structures
- Extend `FormSchema` to describe a **section tree**:
  ```rust
  pub struct FormSchema {
      pub roots: Vec<RootSection>;
      // title/description stay unchanged
  }
  pub struct RootSection {
      pub id: String,
      pub title: String,
      pub description: Option<String>,
      pub sections: Vec<FormSectionNode>, // includes the “General” pseudo-section
  }
  pub struct FormSectionNode {
      pub id: String,
      pub title: String,
      pub description: Option<String>,
      pub path: Vec<String>,
      pub fields: Vec<FieldSchema>,
      pub children: Vec<FormSectionNode>,
  }
  ```
- `General` remains a pseudo-root; everything else is derived from the root-level property names (first path segment).

### 3.2 Parser strategy
1. Iterate root `properties` in order (the `IndexMap` already guarantees that). For each property:
   - If it is a scalar/composite field → append to the `General` section (root id `"general"`).
   - If it is an object with nested properties → create (or reuse) a `RootSection` keyed by the property name and recursively populate its children.
2. During recursion, keep an explicit stack of `(root_id, ancestor_path)` so that we always know which root a nested section belongs to.
3. When we descend into nested objects, push the section node onto the tree but **do not** clone the container as a field. Only scalar/array/composite leaves become `FieldSchema`s.
4. When handling `additionalProperties`, check the object metadata:
   - If the parent is already a dedicated section (e.g., metadata), attach the key-value field to that section rather than reusing the parent path as a new `section_id`.
5. Preserve field order by recording an `ordinal` (usize) when we push each `FieldSchema`. When constructing `SectionState`, sort by this ordinal.

### 3.3 Composite variants with `$ref`
- Store a reference back to the root schema inside `CompositeState`. Instead of serialising the variant to JSON and reparsing, pass the same `SchemaContext` (or a lightweight clone containing `raw` + `definitions`). Two viable options:
  1. Augment `CompositeVariant` with `full_schema: Value` that includes the original `definitions` map when serialised.
  2. Add a helper `SchemaContext::with_root(raw_root: &Value)` accessible from `CompositeState` so variants can resolve `$ref` through the parent context.
- The chosen fix should keep `CompositeEditorSession` self-contained but `$ref`-aware.

### 3.4 FormState & navigation
- Replace the single `Vec<SectionState>` with:
  ```rust
  pub struct FormState {
      pub roots: Vec<RootSectionState>,
      pub root_index: usize,
      pub child_index: usize,
  }
  pub struct RootSectionState {
      pub id: String,
      pub title: String,
      pub sections: Vec<SectionState>,
  }
  ```
- `focus_next_section(delta)` now operates within the current root’s `sections`. New helpers `focus_next_root(delta)` and `sync_child_bounds()` keep indices valid.
- Input mapping:
  - Existing `Ctrl+Tab`/`Ctrl+Shift+Tab` keep calling `KeyCommand::SwitchSection` (only child sections).
  - Introduce `KeyCommand::SwitchRoot(i32)` triggered by `Ctrl+]` / `Ctrl+[`. Update `input::classify` accordingly.

### 3.5 Rendering & keyboard
- Tabs row 1: render `form_state.roots` in order, highlighting `root_index`.
- Tabs row 2: render the active root’s child sections.
- Field renderer: already refactored earlier to follow the `name → value → type → error` order. Once section ordering changes, both main and overlay will gain consistent layouts automatically.

## 4. Implementation Plan
1. **Introduce section tree types**
   - Update `FormSchema`, `FormSection`, `FormState` to the new hierarchical structs.
   - Provide conversion helpers (`RootSectionState::from`, `SectionState::from_node`).
2. **Rewrite parser output**
   - Maintain an `IndexMap<String, RootSectionBuilder>` keyed by the root path (first segment) plus `general`.
   - Each builder holds `IndexMap<Vec<String>, SectionNodeBuilder>` preserving insertion order for child sections.
   - Modify `parse_object_fields` to receive additional context: `(root_id, parent_path, order_counter)`. Only append `FieldSchema`s when hitting leaf nodes.
   - When hitting `additionalProperties`, attach the generated `FieldSchema` to the same section as its parent (no new `section_id`).
3. **Integrate with FormState/UI**
   - Replace the flat section vector with `roots`. Update all methods in `FormState` (`focused_field`, `focus_next_field`, `focus_prev_field`, etc.) to look up the active root first.
   - Modify `render_body` to draw two tab rows and pass the active `SectionState` to `render_fields`.
   - Extend `KeyCommand` with `SwitchRoot(i32)`; update `App::handle_key` to respond by switching `root_index` and resetting `child_index`.
4. **Fix composite `$ref` resolution**
   - Store a clone of the root schema (or a shared `Arc<Value>`) within `CompositeState`. When opening a variant editor, call `parse_form_schema_with_root(&root_value, &variant_value)` that reuses the same `SchemaContext` so `#/definitions/...` lookups succeed.
5. **Regression tests / validation**
   - Manual: `cargo run` and verify the sample schema renders root tabs exactly as `General → Metadata → Runtime → DataPlane → Notifications → FeatureFlags → Secrets` with nested tabs (Http, Cors, Limits) only visible when Runtime is active.
   - Verify Document Store overlay now opens without “unsupported schema” errors.
   - Run `cargo check` after each major step.

This plan keeps the implementation modular (parser, form state, UI, composite context) and honours the KISS principle while enabling the richer TUI requirements.
