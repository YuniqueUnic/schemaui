# schemaui

`schemaui` turns JSON Schema documents into fully interactive terminal UIs
powered by `ratatui`, `crossterm`, and `jsonschema`. The library parses rich
schemas (nested sections, `$ref`, `oneOf`/`anyOf`, arrays, key/value maps,
pattern properties…) into a navigable form tree, renders it as a keyboard-first
editor, and validates the result after every edit so users always see the full
list of issues before saving.

## Feature Highlights

- **Schema fidelity** – draft-07 compatible, including `$ref`, `definitions`,
  `oneOf`/`anyOf`, `patternProperties`, enums, numeric ranges, and deeply nested
  objects/arrays.
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
schemaui = "0.1.1"
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

## Input & Output Layer

- `io::input::parse_document_str` converts JSON/YAML/TOML (via `serde_json`,
  `serde_yaml`, `toml`) into `serde_json::Value`. Feature flags (`json`, `yaml`,
  `toml`, `all_formats`) keep dependencies lean.
- `schema_from_data_value/str` infers a schema from a concrete config snapshot,
  adding `$schema` and `default` hints everywhere.
- `schema_with_defaults` merges a canonical schema with user data, propagating
  defaults through `properties`, `patternProperties`, `additionalProperties`,
  and `$ref` targets.
- `io::output::OutputOptions` controls the serialization format, prettiness, and
  a list of `OutputDestination` (stdout or file paths). Multiple destinations
  are supported; conflicts are reported unless `--force` is used in the CLI.

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
  `std::sync::LazyLock` parses it once at startup, and each combo is compiled
  into a `KeyPattern` (key code, required modifiers, pretty display string).
- **Integration** – `InputRouter::classify` delegates to `keymap::classify_key`,
  which returns the `KeyAction` embedded in the JSON. `keymap::help_text`
  filters bindings by `KeymapContext`, concatenating snippets used by
  `StatusLine` and overlay instructions.
- **Extending** – to add a shortcut, edit the JSON, choose the contexts that
  should expose the help text, and wire the resulting `KeyAction` inside
  `KeyBindingMap` if a new semantic command is introduced.

## CLI (`schemaui-cli`)

```
schemaui \
  --schema ./schema.json \
  --config ./defaults.yaml \
  -o ./config.toml ./config.json -o -
```

- `--schema SPEC`, `--config SPEC` accept file paths, inline JSON/TOML/YAML
  blobs, or `-` for stdin. If both streams need piping, send one as inline text.
- The CLI aggregates I/O errors so users see every malformed input/output path
  at once instead of failing fast.
- `-o/--output DEST...` may be repeated; pass `-` to include stdout. When no
  explicit destination is present, the tool falls back to a temp file unless
  `--no-temp-file` is set. Formats are inferred from the first file extension or
  the input hints; a warning is emitted if the requested format is gated behind
  a disabled feature.
- `--force`/`--yes` allows overwriting existing files. Without it, collisions
  are reported through the shared diagnostic collector.

## Development

- Run `cargo fmt && cargo test` regularly; most modules embed their tests by
  `include!`ing files from `tests/` so private APIs stay covered.
- `structure_design.md` documents the full architecture (I/O logic, schema
  pipeline, runtime layering, overlays, validation strategy, keyboard map). Keep
  that file up-to-date when adding new features.

Happy hacking!
