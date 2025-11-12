# schemaui

[![Crates.io](https://img.shields.io/crates/v/schemaui.svg)](https://crates.io/crates/schemaui)
[![Documentation](https://docs.rs/schemaui/badge.svg)](https://docs.rs/schemaui)
[![License](https://img.shields.io/crates/l/schemaui)](https://github.com/yuniqueunic/schemaui#license)
![Crates.io Total Downloads](https://img.shields.io/crates/d/schemaui)
![Deps.rs Crate Dependencies (latest)](https://img.shields.io/deps-rs/schemaui/latest)

<div align="center">
<img src="https://github.com/YuniqueUnic/schemaui/blob/master/docs/schemaui-demo.gif?raw=true" width="500">

[English](./README.md) | [中文文档](./README.ZH.md)

</div>

`schemaui` turns JSON Schema documents into fully interactive terminal UIs
powered by `ratatui`, `crossterm`, and `jsonschema`.

The library parses rich schemas (nested sections, `$ref`, arrays, key/value
maps, pattern properties…) into a navigable form tree, renders it as a
keyboard-first editor, and validates the result after every edit so users always
see the full list of issues before saving.

## Feature Highlights

- **Schema fidelity** – draft-07 compatible, including `$ref`, `definitions`,
  `patternProperties`, enums, numeric ranges, and nested objects/arrays.
- **Sections & overlays** – top-level properties become root tabs, nested
  objects are flattened into sections, and complex nodes (composites, key/value
  collections, array entries) open dedicated overlays with their own validators.
- **Immediate validation** – every keystroke can trigger
  `jsonschema::Validator`, and all errors (field-scoped + global) are collected
  and displayed together.
- **Pluggable I/O** – `io::input` ingests JSON/YAML/TOML (feature-gated) while
  `io::output` can emit to stdout and/or multiple files in any enabled format.
- **Batteries-included CLI** – `schemaui-cli` offers the same pipeline as the
  library, including multi-destination output, stdin/inline specs, and
  aggregated diagnostics.

## Quick Start

```toml
[dependencies]
schemaui = "0.3.1"
serde_json = "1"
```

```rust
use schemaui::SchemaUI;
use serde_json::json;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Service Runtime",
        "type": "object",
        "properties": {
            "metadata": {
                "type": "object",
                "properties": {
                    "serviceName": {"type": "string"},
                    "environment": {
                        "type": "string",
                        "enum": ["dev", "staging", "prod"]
                    }
                },
                "required": ["serviceName"]
            },
            "runtime": {
                "type": "object",
                "properties": {
                    "http": {
                        "type": "object",
                        "properties": {
                            "host": {"type": "string", "default": "0.0.0.0"},
                            "port": {"type": "integer", "minimum": 1024, "maximum": 65535}
                        }
                    }
                }
            }
        },
        "required": ["metadata", "runtime"]
    });

    let value = SchemaUI::new(schema)
        .with_title("SchemaUI Demo")
        .run()?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
```

## Architecture Snapshot

```
┌─────────────┐   parse/merge    ┌───────────────┐   layout + typing   ┌─────────────┐
│ io::input   ├─────────────────▶│ schema::*     ├────────────────────▶│ form::*     │
└─────────────┘                  │ (loader /     │                     │ (state,     │
                                 │ resolver /    │                     │ sections,   │
┌─────────────┐   emit Value     │ layout)       │   FormState         │ reducers)   │
│ io::output  ◀──────────────────┴───────────────┘                     └────────┬────┘
└─────────────┘                                                      focus/edits│
                                                                                │
                                                                     ┌──────────▼────────┐
                                                                     │ app::runtime      │
                                                                     │ (InputRouter,     │
                                                                     │ overlays, status) │
                                                                     └──────────┬────────┘
                                                                                │ draw
                                                                     ┌──────────▼────────┐
                                                                     │ presentation::*   │
                                                                     │ (ratatui view)    │
                                                                     └───────────────────┘
```

This layout mirrors the actual modules under `src/`, making it easy to map any
code change to its architectural responsibility.

## Input & Output Design

- `io::input::parse_document_str` converts JSON/YAML/TOML (via `serde_json`,
  `serde_yaml`, `toml`) into `serde_json::Value`. Feature flags (`json`, `yaml`,
  `toml`, `all_formats`) keep dependencies lean.
- `schema_from_data_value/str` infers schemas from live configs, injecting
  draft-07 metadata and defaults so UIs load pre-existing values.
- `schema_with_defaults` merges canonical schemas with user data, propagating
  defaults through `properties`, `patternProperties`, `additionalProperties`,
  `dependencies`, `dependentSchemas`, arrays, and `$ref` targets without
  mutating the original tree.
- `io::output::OutputOptions` encapsulates serialization format, pretty/compact
  toggle, and a vector of `OutputDestination::{Stdout, File}`. Multiple
  destinations are supported; conflicts are caught before emission.
- `SchemaUI::with_output` wires these options into the runtime so the final
  `serde_json::Value` can be written automatically after the session ends.

## JSON Schema → TUI Mapping

`schema::layout::build_form_schema` walks the fully resolved schema and maps
each sub-tree to a `FormSection`/`FieldSchema`:

| Schema feature                                               | Resulting control                                                                |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| `type: string`, `integer`, `number`                          | Inline text editors with numeric guards                                          |
| `type: boolean`                                              | Toggle/checkbox                                                                  |
| `enum`                                                       | Popup selector (single or multi-select for array enums)                          |
| Arrays                                                       | Inline list summary + overlay editor per item                                    |
| `patternProperties`, `propertyNames`, `additionalProperties` | Key/Value editor with schema-backed validation                                   |
| `$ref`, `definitions`                                        | Resolved before layout; treated like inline schemas                              |
| `oneOf` / `anyOf`                                            | Variant chooser + overlay form, keeps inactive variants out of the final payload |

Root objects spawn tabs; nested objects become sections with breadcrumb titles.
Every field records its JSON pointer (for example `/runtime/http/port`) so focus
management and validation can map errors back precisely.

## Validation Lifecycle

- `jsonschema::validator_for` compiles the complete schema once when
  `SchemaUI::run` begins.
- Each edit dispatches `FormCommand::FieldEdited`. `FormEngine` rebuilds the
  current document via `FormState::try_build_value`, runs the validator, and
  feeds errors back into `FieldState` or the global status line.
- Overlays (composite variants, key/value maps, list entries) spin up their own
  validators built from the sub-schema currently being edited, so issues surface
  before leaving the overlay.

```
┌─────────────┐ parse schema ┌─────────────────┐ inflate state  ┌────────────┐
│ SchemaUI::run├────────────▶│ domain::parse   ├───────────────▶│ FormState  │
└─────┬───────┘              │ (schema::layout)│                └─────┬──────┘
      │ validator_for()      └─────────────────┘                edits │
      │                                                        ┌──────▼─────────┐
      └────────────────────────────────────────────────────── ▶│ app::runtime   │
                                                               │ (status, input)│
                                                               └──────┬─────────┘
                                                                      │ FormCommand
                                                               ┌──────▼──────────┐
                                                               │ FormEngine      │
                                                               │ + jsonschema    │
                                                               └─────────────────┘
```

`App` is the sole owner of `FormState`; even overlay edits flow through
`FormEngine` so validation rules stay centralized.

## TUI Building Blocks & Shortcuts

- **Single source for shortcuts** – `keymap/default.keymap.json` lists every
  shortcut (context, combos, action). The `app::keymap::keymap_source!()` macro
  pulls this file into the binary, `InputRouter` uses it to classify
  `KeyEvent`s, and the runtime footer renders help text from the same
  data—keeping docs and behavior DRY.
- **Root tabs & sections** – focus cycles with `Ctrl+J / Ctrl+L` (roots) and
  `Ctrl+Tab / Ctrl+Shift+Tab` (sections). Ordinary `Tab`/`Shift+Tab` walk
  individual fields.
- **Fields** – render labels, descriptions, and inline error messages.
  Enum/composite fields show the current selection; arrays summarize length and
  selected entry.
- **Popups & overlays** – pressing `Enter` opens a popup for enums/oneOf
  selectors; `Ctrl+E` opens the full-screen overlay editor for composites,
  key/value pairs, and array items. Overlays expose collection shortcuts
  (`Ctrl+N`, `Ctrl+D`, `Ctrl+←/→`, `Ctrl+↑/↓`) plus `Ctrl+S` to commit.
- **Status & help** – the footer highlights dirty state, outstanding validation
  errors, and context-aware help text. When auto-validate is enabled, each edit
  updates these counters immediately.

| Context     | Shortcut                            | Action                                |
| ----------- | ----------------------------------- | ------------------------------------- |
| Navigation  | `Tab` / `Shift+Tab`                 | Move between fields                   |
|             | `Ctrl+Tab` / `Ctrl+Shift+Tab`       | Switch sections                       |
|             | `Ctrl+J` / `Ctrl+L`                 | Switch root tabs                      |
| Selection   | `Enter`                             | Open popup / apply choice             |
| Editing     | `Ctrl+E`                            | Launch composite editor               |
| Status      | `Esc`                               | Clear status or close popup           |
| Persistence | `Ctrl+S`                            | Save + validate                       |
| Exit        | `Ctrl+Q` / `Ctrl+C`                 | Quit (requires confirmation if dirty) |
| Collections | `Ctrl+N` / `Ctrl+D`                 | Add / remove entry                    |
|             | `Ctrl+←/→`, `Ctrl+↑/↓`              | Select / reorder entries              |
| Overlay     | `Ctrl+S`, `Esc`, `Ctrl+N/D/←/→/↑/↓` | Save, cancel, manage composite lists  |

### Keymap system

Put every shortcut into `keymap/default.keymap.json`, so runtime logic, help
overlays, and documentation all consume a single source of truth.

- **Format** – each JSON object declares an `id`, human-readable `description`,
  `contexts` (any of `"default"`, `"collection"`, `"overlay"`), an `action`
  discriminated union, and a list of textual `combos`. For example:

  ```json
  {
    "id": "list.move.up",
    "description": "Move entry up",
    "contexts": ["collection", "overlay"],
    "action": { "kind": "ListMove", "delta": -1 },
    "combos": ["Ctrl+Up"]
  }
  ```

- **Macro + parser** – `app::keymap::keymap_source!()` `include_str!`s the JSON,
  `once_cell::sync::Lazy` parses it once at startup, and each combo is compiled
  into a `KeyPattern` (key code, required modifiers, pretty display string).
- **Integration** – `InputRouter::classify` delegates to `keymap::classify_key`,
  which returns the `KeyAction` embedded in the JSON. `keymap::help_text`
  filters bindings by `KeymapContext`, concatenating snippets used by
  `StatusLine` and overlay instructions.
- **Extending** – to add a shortcut, edit the JSON, choose the contexts that
  should expose the help text, and wire the resulting `KeyAction` inside
  `KeyBindingMap` if a new semantic command is introduced.

## Runtime Layers

| Layer               | Module(s)                                                                  | Responsibilities                                                                |
| ------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Ingestion           | `io::input`, `schema::loader`, `schema::resolver`                          | Parse JSON/TOML/YAML, resolve `$ref`, and normalize metadata.                   |
| Layout typing       | `schema::layout`                                                           | Produce `FormSchema` (roots/sections/fields) from resolved schemas.             |
| Form state          | `form::state`, `form::section`, `form::field`                              | Track focus, pointers, dirty flags, coercions, and errors.                      |
| Commands & reducers | `form::actions`, `form::reducers`, `app::validation`                       | Define `FormCommand`, mutate state, and route validation results.               |
| Runtime controller  | `app::runtime`, `app::overlay`, `app::popup`, `app::status`, `app::keymap` | Event loop, InputRouter dispatch, overlay lifecycle, help text, status updates. |
| Presentation        | `presentation::view`, `presentation::components::*`                        | Render tabs, field lists, popups, overlays, and footer via `ratatui`.           |

Each module is kept under ~600 LOC (hard cap 800) to honor the KISS principle
and make refactors manageable.

## CLI (`schemaui-cli`)

```bash
cargo install schemaui-cli
# It will be installed to `~/.cargo/bin` and renamed to `schemaui`
# so you should use it like this: `schemaui -c xxx`
```

```bash
schemaui \
  --schema ./schema.json \
  --config ./defaults.yaml \
  -o - \
  -o ./config.toml ./config.json
```

```
┌────────┐  clap args   ┌──────────────┐ read stdin/files ┌─────────────┐
│  CLI   ├─────────────▶│ InputSource  ├─────────────────▶│ io::input   │
└────┬───┘              └──────┬───────┘                  └────┬────────┘
     │ diagnostics             │ schema/default Value          │
┌────▼─────────┐        ┌──────▼──────┐                        |
│Diagnostic    │◀───────┤ FormatHint  │                        │
│Collector     │        └──────┬──────┘                        │
└────┬─────────┘               │ pass if clean                 │
     │                         │                               │
┌────▼────────┐  build options └────────────┐                  │
│Output logic ├────────────────────────────▶│ OutputOptions    │
└────┬────────┘                             └────────────┬─────┘
     │ SchemaUI::new / with_*                        ┌───▼────────┐
     └──────────────────────────────────────────────▶│ SchemaUI   │
                                                     │ (library)  │
                                                     └────────────┘
```

- Inputs – `--schema` / `--config` accept file paths, inline payloads, or `-`
  for stdin (but not both simultaneously). If only config is provided the CLI
  infers a schema via `schema_from_data_value`.
- Diagnostics – `DiagnosticCollector` accumulates format issues, feature flag
  mismatches, stdin conflicts, and existing output files before execution.
- Outputs – `-o/--output` is repeatable and may mix file paths with `-` for
  stdout. When no destination is set, the tool writes to `/tmp/schemaui.json`
  unless `--no-temp-file` is passed. Extensions dictate formats; conflicting
  extensions are rejected.
- Flags – `--no-pretty` toggles compact output, `--force/--yes` allows
  overwriting files, and `--title` wires through to `SchemaUI::with_title`.

## Key Dependencies

| Crate                                       | Purpose                                                  |
| ------------------------------------------- | -------------------------------------------------------- |
| `serde`, `serde_json`, `serde_yaml`, `toml` | Parsing and serializing schema/config data.              |
| `schemars`                                  | Draft-07 schema representation used by `schema::layout`. |
| `jsonschema`                                | Runtime validation for forms and overlays.               |
| `ratatui`                                   | Rendering widgets, layouts, overlays, and footer.        |
| `crossterm`                                 | Terminal events consumed by `InputRouter`.               |
| `indexmap`                                  | Order-preserving maps for schema traversal.              |
| `once_cell`                                 | Lazy parsing of the keymap JSON.                         |
| `clap`, `color-eyre` (CLI)                  | Argument parsing and ergonomic diagnostics.              |

## Documentation Map

- `README.md` – overview + architecture snapshot.
- `docs/en/structure_design.md` – detailed schema/layout/runtime design with
  flow diagrams.
- `docs/en/cli_usage.md` – CLI-specific manual (inputs, outputs, piping,
  samples).

## Development

- Run `cargo fmt && cargo test` regularly; most modules embed their tests by
  `include!`ing files from `tests/` so private APIs stay covered.
- Keep modules below ~600 LOC (hard cap 800). Split helpers as soon as behavior
  grows to keep KISS intact.
- Prefer mature crates (`serde_*`, `schemars`, `jsonschema`, `ratatui`,
  `crossterm`) over bespoke code unless the change is trivial.
- Update `docs/*` whenever pipelines, shortcuts, or CLI semantics evolve so
  user-facing documentation stays truthful.

## References

1. https://github.com/rjsf-team/react-jsonschema-form
2. https://ui-schema.bemit.codes/examples

## Roadmap

- [x] parse json schema at runtime and generate a TUI
- [ ] parse json schema at runtime and generate a Web UI
- [ ] parse json schema at compile time Then generate the code for TUI, expose
      necessary APIs for runtime.
- [ ] parse json schema at compile time Then generate the code for Web UI,
      expose necessary APIs for runtime.
- [ ] parse json schema at runtime and generate a Interactive CLI
- [ ] parse json schema at compile time Then generate the code for Interactive
      CLI, expose necessary APIs for runtime.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

Happy hacking!

## Star History

<a href="https://www.star-history.com/#YuniqueUnic/schemaui&type=date&legend=top-left">
<picture>
  <source
    media="(prefers-color-scheme: dark)"
    srcset="
      https://api.star-history.com/svg?repos=YuniqueUnic/schemaui&type=date&legend=top-left&theme=dark
    "
  />
  <source
    media="(prefers-color-scheme: light)"
    srcset="
      https://api.star-history.com/svg?repos=YuniqueUnic/schemaui&type=date&legend=top-left
    "
  />
  <img
    alt="Star History Chart"
    src="https://api.star-history.com/svg?repos=YuniqueUnic/schemaui&type=date&legend=top-left"
  />
</picture>
</a>
