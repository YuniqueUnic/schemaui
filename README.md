## schemaui

**schemaui** turns JSON Schema definitions into a terminal-first configuration experience.  
It parses schemas (including `$ref`, `oneOf`, `anyOf`, nested objects, arrays, key/value maps, etc.), maps them into a focusable form tree, renders a Ratatui-based UI, and performs JSON Schema validation after every edit.

### Quick start

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
                    "environment": {"type": "string", "enum": ["dev", "staging", "prod"]}
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

    let value = SchemaUI::new(schema).with_title("SchemaUI Demo").run()?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
```

### Features at a glance

- **Rich schema support**: nested roots/sections, composites (`oneOf`/`anyOf`), scalar & composite lists, key/value maps, enums, `$ref` chains, pattern properties.
- **Deterministic mapping pipeline**: loader → resolver → layout builder → `FormState` (see [structure_design.md](structure_design.md)).
- **Live validation**: every successful edit runs through `jsonschema::Validator`. Errors are surfaced inline and summarized in the status line.
- **Overlay editors**: complex fields (composites, arrays, key/value) open as focused overlays with their own form state and validator.
- **Keyboard-first UX**: input routing maps key chords to semantic actions so the UI stays responsive in every context.
- **Color-eyre integration**: panic hooks restore the terminal and render rich diagnostics when something goes wrong.

### Schema coverage

- **Multiple roots & sections** – each top-level property becomes a root tab; nested objects become sections (recursively) with breadcrumb-style labels.
- **Nested structures** – objects/arrays can nest arbitrarily; scalar arrays render as comma lists, composite arrays/key/value maps open repeatable editors.
- **References** – `$ref` (definitions + JSON pointers) are resolved before layout, so referenced schemas behave exactly like inline definitions.
- **Composites** – `oneOf`/`anyOf` render as variant selectors; picking a variant opens an overlay bound to the chosen subschema.
- **Validation keywords** – type, enum, numeric ranges, `patternProperties`, `additionalProperties:false`, etc., are enforced both during editing and on save.

### Validation & feedback loop

1. User edits a field (main form or overlay).
2. `FormState` marks the field dirty and dispatches `FormCommand::FieldEdited`.
3. `FormEngine` rebuilds the JSON value and feeds it to `jsonschema::Validator`.
4. Errors for the focused pointer are written back to `FieldState::error`, immediately reflected in the UI (red annotations + status line message).
5. Saving reruns the validator against the entire form; passing validation yields the final JSON value.

### TUI composition

- **Root tabs** across the top show each schema root; `Ctrl+[ / Ctrl+]` or `Ctrl+J / Ctrl+L` (terminal-friendly) switches roots.
- **Section tabs** track nested groups. Navigation (`Tab`, `Shift+Tab`, arrow keys) wraps seamlessly across sections and roots.
- **Field list** renders labels, selectors, summaries, type metadata, and validation errors.
- **Status/help footer** displays current action hints, dirty state, and validation summaries.
- **Overlays** (composites, arrays, key/value) are mini forms with their own list panel, instructions, and validator; they never close unless the user commits (`Ctrl+S`) or explicitly cancels (`Esc` twice when dirty).

### Keyboard shortcuts

| Context | Keys | Action |
| --- | --- | --- |
| Navigation | `Tab` / `Shift+Tab` | Move across fields/sections (wraps across roots) |
|  | `Ctrl+Tab` / `Ctrl+Shift+Tab` | Jump entire sections (also wraps roots) |
|  | `Alt+Shift+[ / ]` or `Alt+Shift+← / →` | Switch root tabs |
| Field interaction | `Enter` | Open popup or variant selector |
|  | `Esc` | Reset status / close popup (twice to discard overlays) |
| Persistence | `Ctrl+S` | Save + validate |
|  | `Ctrl+Q` | Quit (prompts when dirty) |
| Collections & maps | `Ctrl+N` / `Ctrl+D` | Add / remove entry |
|  | `Ctrl+←` / `Ctrl+→` | Select previous/next entry |
|  | `Ctrl+↑` / `Ctrl+↓` | Reorder entry |
| Composite overlay | `Ctrl+E` | Open editor for composite / complex field |
| Overlay specific | `Ctrl+S` / `Esc` | Commit overlay / close overlay (double `Esc` to discard) |
|  | `Tab` / `Shift+Tab` | Navigate overlay fields |

### Architecture

1. **Schema ingestion** (`schema::loader`, `schema::resolver`, `schema::layout`): parses JSON Schema into a `FormSchema` composed of roots, sections, and typed fields.
2. **Form model** (`form::state`, `form::field`, `form::key_value`, `form::composite`): holds focus, field values, dirty/error flags, and validates edits via `FormEngine`.
3. **Runtime** (`app::runtime` and submodules): event loop, error/status handling, overlay orchestration, persistence, auto-validation, and input routing.
4. **Presentation** (`presentation::components`): Ratatui widgets for roots, sections, fields, overlay panes, and status/footer.

Read the in-depth breakdown in [structure_design.md](structure_design.md).

### Testing

All tests live under `tests/` and mirror the module layout. They are compiled into each module with `include!` so private APIs remain testable.

Run the suite with:

```bash
cargo test
```

### License

Dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE](LICENSE) or <http://opensource.org/licenses/MIT>)

### Contributing

Contributions are welcome! Please:

1. Format & lint (`cargo fmt && cargo check`)
2. Add/extend tests under `tests/…`
3. Describe changes clearly in PRs
