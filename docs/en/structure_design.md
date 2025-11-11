# schemaui Architecture Guide

This document explains how schemaui ingests schemas/configs, maps them onto form
structures, renders TUIs, and validates data. Use it to navigate the codebase
and as a checklist when changing the pipeline.

## 1. Design Principles

1. **Restraint & precision** – keep modules small (prefer <600 LOC), split files
   when behaviour grows, and lean on mature crates (`serde_*`, `jsonschema`,
   `ratatui`, `crossterm`, `clap`).
2. **Schema fidelity** – the runtime must never drop keywords that affect
   validation or UI (draft-07 coverage including `$ref`, `definitions`,
   `patternProperties`, `oneOf/anyOf`).
3. **Form-first rendering** – the TUI only consumes `FormState`; raw JSON is
   parsed once by the IO/schema layers and never again in presentation code.
4. **Full diagnostics** – input and output errors are accumulated (CLI +
   runtime) before being surfaced so users see everything they need to fix in
   one pass.
5. **Keyboard-centric UX** – every action maps to a `KeyAction` →
   `CommandDispatch` chain, keeping navigation, overlays, and popups consistent.

## 2. Data Ingestion & I/O Layer

Module: `src/io` (shared by the library and CLI).

```
┌────────────┐ text/file/stdin ┌───────────────────┐ serde_* per feature ┌──────────────┐
│ Document   ├────────────────▶│ io::input::parse  ├────────────────────▶│ serde_json:: │
│ source     │                 │ (DocumentFormat)  │                     │ Value        │
└────────────┘                 └───────────────────┘                     └──────────────┘
```

- **Input sources** – `io::input::parse_document_str` uses `serde_json`,
  `serde_yaml`, and `toml` (feature gated) to ingest JSON/TOML/YAML from files,
  stdin, or inline strings. The CLI mirrors this logic and first checks whether
  a path exists; if not, the literal string is parsed.
- **Schema + config relationship** – users may pass a canonical schema plus a
  config snapshot. `schema_with_defaults` (powered by `DefaultApplier`) injects
  snapshot values as `default` keywords across `properties`,
  `patternProperties`, `additionalProperties`, `$ref` targets, arrays, and
  dependent schemas without mutating structure. When only data is provided,
  `schema_from_data_value/str` infers a schema and annotates it with defaults.
- **Format hints & features** – `DocumentFormat::available_formats()` reflects
  compile-time features. The CLI’s `FormatHint`/`InputSource` combo inspects
  extensions, rejects requests for disabled formats, and controls stdin usage.
- **Outputs** – `io::output::OutputOptions` groups format selection,
  pretty/compact toggle, and a vector of `OutputDestination::{Stdout, File}`.
  CLI callers may provide multiple destinations (mixing stdout and files) and
  the library reuses the same type via `SchemaUI::with_output`.
- **Diagnostics** – the CLI’s `DiagnosticCollector` retains every issue (invalid
  schema/config spec, mixed output formats, missing features, existing files)
  and reports them together before the UI launches.

## 3. Schema Interpretation Pipeline

```
io::input (serde_json::Value)
  → schema::loader::load_root_schema            // deserializes RootSchema
  → schema::resolver::SchemaResolver            // resolves $ref / JSON Pointer
  → schema::layout::build_form_schema           // builds FormSchema tree
  → form::state::FormState::from_schema         // materializes FieldState
  → app::runtime::App                           // drives TUI + validation
  → io::output::emit (optional)                 // writes final Value
```

Key responsibilities:

1. **Loader** – `serde_json::from_value` with context so users get actionable
   errors when the schema is malformed.
2. **Resolver** – expands `$ref` inside `properties`, `definitions`, or
   arbitrary JSON Pointer fragments, ensuring downstream logic works with fully
   materialized `SchemaObject`s.
3. **Layout** – converts the resolved schema to `FormSchema` by walking each
   object:
   - Top-level properties become `RootSection`s (one per property) plus a
     synthetic “General” root for loose fields.
   - Nested objects become `FormSection`s. `SectionInfo` uses metadata (`title`,
     `description`, or custom `x-group*` extensions) to name sections.
   - `detect_kind` maps `SchemaObject` → `FieldKind` (primitives, enums, arrays,
     composite `oneOf`/`anyOf`, key/value maps) while guarding unsupported
     shapes.
4. **Form state** – `FormState::from_schema` flattens each section, tracks
   `root_index`/`section_index`/`field_index`, and exposes helpers for
   navigation, value assembly (`try_build_value`), seeding defaults, and error
   bookkeeping.
5. **Runtime** – `app::runtime::App` ties together input handling, overlays,
   status messaging, validation, and optional output serialization.

## 4. Supported Schema Structures

The current layout + form stack handles:

- Arbitrary root sections (tabs) and nested sections with breadcrumb titles.
- Deeply nested objects and arrays (arrays of composites + enums open overlays;
  arrays of scalars stay inline).
- `$ref` chains and shared `definitions`.
- `oneOf` / `anyOf` composites (single- or multi-select depending on schema).
  Users select a variant via popup then edit the expanded content inside an
  overlay.
- `patternProperties`, `propertyNames`, and `additionalProperties` for building
  schema-backed key/value editors.
- `dependentSchemas`, `dependencies`, and defaults inserted through
  `schema_with_defaults` so derived values surface instantly.

Unsupported shapes (e.g., nested arrays-of-arrays) log user-facing errors during
layout so they can adjust the schema instead of encountering undefined
behaviour.

## 5. Validation & Error Surfacing

File: `src/app/validation.rs` + `form::reducers`.

1. `SchemaUI::run` compiles a `jsonschema::Validator` up front (panics become
   `color-eyre` reports with context).
2. Each edit emits `FormCommand::FieldEdited { pointer }`. The `FormEngine`
   reconstructs the JSON value via `FormState::try_build_value` and feeds it
   into the validator.
3. `ValidationOutcome::Invalid` clears old errors, distributes new ones to
   matching fields (by JSON pointer) via `FormState::set_error`, and copies the
   rest into the global error list (rendered beneath the footer).
4. Build failures (e.g., invalid number literal) result in
   `ValidationOutcome::BuildError`, keeping the validator intact but
   highlighting the offending field.
5. Overlays use `validator_for` on the sub-schema that corresponds to the
   current composite/key-value/list entry, ensuring nested edits are also
   validated before committing.

## 6. Runtime & Presentation Layering

- **Input handling** – `app::input::InputRouter` classifies `KeyEvent`s into
  semantic `KeyAction`s (field step, section step, root step, popup toggle, list
  operations, overlay edit, save/quit). `KeyBindingMap` converts the action into
  a `CommandDispatch::{Form, App, Input}`. Custom bindings can be injected
  through `UiOptions`.
- **Keymap pipeline** – Introduces `keymap/default.keymap.json`, parsed by
  `app::keymap` once via `once_cell::sync::Lazy`. Each entry contains:
  - `id` + `description` for docs/help text.
  - `contexts`: any of `default`, `collection`, `overlay`. These map to
    `KeymapContext` so `keymap::help_text` can render the right footer snippet.
  - `action`: tagged object (`Save`, `FieldStep { delta }`,
    `ListMove { delta }`, etc.) that deserializes directly into `KeyAction`.
  - `combos`: textual shortcuts (e.g., `"Ctrl+Shift+Tab"`). Tokens are parsed
    into `KeyPattern`s (required modifiers + code matcher). Letter combos
    implicitly tolerate `Shift` unless the pattern already requires it.
    `InputRouter::classify` now defers entirely to `keymap::classify_key`, and
    the overlay/status modules pull help text from the same dataset,
    guaranteeing DRY docs + UI. Adding a shortcut therefore only requires
    editing the JSON file (and optionally `KeyBindingMap` if a brand new
    `KeyAction` is introduced).
- **App runtime** – `app::runtime::App` maintains:
  - `FormState` and the compiled validator
  - `StatusLine` (dirty flag, help text, diagnostic count)
  - `PopupState` for enum/variant selection
  - `CompositeEditorOverlay` sessions (see `runtime/overlay.rs`) for editing
    nested data with undo/redo semantics and per-entry validators
  - List helpers (`runtime/list_ops.rs`) for insert/delete/reorder operations
    shared by both the main view and overlays
  - A central draw loop using `TerminalGuard` (restores terminal state on panic)
- **Presentation** – `presentation::view` splits the screen into body + footer,
  then calls `components::*` to render:
  - Root & section tabs with focus markers
  - Field rows (label, value preview, metadata badge, inline error message)
  - Popups (enum/variant pickers) and overlays (full-screen editors with
    optional list panels)
  - Footer / help overlay (dirty state, validation count, context-aware hint)

### 6.1 Event loop timeline

```
KeyEvent (crossterm)
    │
    ▼
InputRouter::classify ─▶ KeyAction ─▶ KeyBindingMap ─▶ CommandDispatch
                                                          │
                                                          ▼
                                                  app::runtime::App
                                                          │
                                                          ▼
                                                FormEngine + Validator
                                                          │
                                                          ▼
                                                  presentation::draw
```

- `InputRouter` defers to `keymap::classify_key`, keeping the key matrix in
  JSON.
- `KeyBindingMap` translates semantic actions into either `FormCommand` (mutate
  `FormState`) or `AppCommand` (popups, overlays, status, quit/save).
- `App::handle_key` routes all side effects before scheduling the next
  `presentation::draw` call.

### 6.2 Overlay lifecycle

```
Form focus ──Ctrl+E──▶ try_open_composite_editor
                          │
                          ▼
                 CompositeEditorOverlay::new
                          │ setup_overlay_validator
                          ▼
                   overlay FormState + StatusLine
                          │
        (InputRouter + KeyBindingMap reused inside overlay)
                          │
                          ▼
                      save/quit ──▶ close_composite_editor(commit)
```

- Overlays spawn their own `FormState` and optional list panel metadata while
  reusing the global validator via `jsonschema::validator_for` scoped to the
  nested schema.
- Help text is sourced through `keymap::help_text(KeymapContext::Overlay)` so
  footer messaging stays synchronized.

## 7. Shortcut Reference

| Scope       | Shortcut                                   | Command                                             |
| ----------- | ------------------------------------------ | --------------------------------------------------- |
| Fields      | `Tab` / `Shift+Tab`, `Down` / `Up`         | Cycle within a section                              |
| Sections    | `Ctrl+Tab` / `Ctrl+Shift+Tab`              | Jump between sections in the current root           |
| Roots       | `Ctrl+J` / `Ctrl+L`                        | Jump between root tabs                              |
| Popups      | `Enter` (open/apply), `Esc` (close/reset)  | Manage enums/composites                             |
| Lists       | `Ctrl+N`, `Ctrl+D`, `Ctrl+←/→`, `Ctrl+↑/↓`| Add/delete/select/reorder entries                   |
| Overlay     | `Ctrl+E`                                   | Launch editor for composites/key-value/list entries |
| Persistence | `Ctrl+S`                                   | Save + validate                                     |
| Exit        | `Ctrl+Q`, `Ctrl+C`                         | Arm quit / confirm quit                             |

Every shortcut runs through `InputRouter` so overlay and main views behave
identically unless explicitly overridden.

## 8. CLI Flow (schemaui-cli)

1. Parse `--schema SPEC` and `--config SPEC`. Each spec can be a file path, raw
   payload, or `-` (stdin). Both streams cannot use stdin simultaneously; users
   are prompted to send one inline instead.
2. Determine format hints from extensions, check feature availability, and
   attempt to load/parse. Failures push messages into `DiagnosticCollector`
   rather than returning early.
3. Build output destinations from repeated `-o/--output` arguments (multiple
   values per occurrence are accepted). Destinations can mix stdout (`-`) and
   file paths; a ny conflicting extensions, missing features, or existing files
   are recorded.
4. After all diagnostics are reported (if any), the CLI instantiates `SchemaUI`,
   seeds defaults (if config provided), runs the TUI, and finally emits results
   using the requested format + destinations.

```
┌────────┐ args┌───────────────┐ schema/config ┌──────────────┐ result ┌────────────┐
│  clap  ├────▶│ InputSource   ├──────────────▶│ SchemaUI     ├──────▶│ io::output │
└────┬───┘     └─────────┬─────┘               │ (library)    │        └────┬───────┘
     │ diagnostics       │ format hint         └─────┬────────┘             │  writes
┌────▼─────────┐         │ DocumentFormat            │ validator            ▼  files/stdout
│Diagnostic    │◀────────┘ (extension or default)    │                      
│Collector     │                                     ▼                      
└──────────────┘                               Interactive UI       
```

## 9. Public APIs & Output Hooks

- **`SchemaUI`** (`src/app/schema_ui.rs`) – entry point for library consumers.
  Exposes constructors for raw schema values, schema+data pairs, and inferred
  schemas. Chain `.with_title`, `.with_options`, `.with_output`, or
  `.with_default_data` before calling `.run()`.
- **`UiOptions`** – toggles UI behaviour (tick rate, auto validation, help
  visibility, custom key bindings via `KeyBindingMap`).
- **`OutputOptions` + `OutputDestination`** – configure format, prettiness, and
  destinations. Shared by the CLI and arbitrary host applications.
- **`DocumentFormat::available_formats()`** – reveals which
  parsing/serialization capabilities were compiled in so hosts can tailor UX.

## 10. Testing & Maintenance

- Module-specific tests live under `tests/` and are `include!`d into their
  respective modules for private API coverage. Add new test files instead of
  growing existing ones past ~200 lines.
- Run `cargo check` or `cargo test -p schemaui-cli` before committing; large
  refactors should cover both the library and CLI crates.
- Keep documentation bilingual? No—public docs (README/design) must stay in
  English so downstream users can consume them without translation.

Refer back to this guide whenever you add schema keywords, introduce new
overlays, or modify CLI semantics.
