# schemaui-cli Usage Guide

`schemaui-cli` is the official command-line wrapper around the `schemaui`
library. It accepts JSON Schema + config snapshots, defaults to the interactive
TUI when no mode subcommand is provided, and also exposes explicit `tui`, `web`,
`tui-snapshot`, and `web-snapshot` subcommands. This guide mirrors the actual
code in `schemaui-cli/src/main.rs` so the behaviour stays predictable.

## 1. Install & Run

### From source

```bash
cargo run -p schemaui-cli -- tui --schema ./schema.json --config ./config.yaml
```

### Install as a binary

```bash
cargo install schemaui-cli
schemaui --help             # binary is named `schemaui` via the clap metadata
```

If you omit a mode subcommand, `schemaui` falls back to the TUI flow, so the
following invocations are equivalent:

```bash
schemaui --schema ./schema.json
schemaui tui --schema ./schema.json
```

## 2. Execution Flow

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

Key components:

- **`InputSource`** – resolves files vs stdin vs inline specs.
- **`FormatHint`** – inspects extensions and ensures disabled formats are
  rejected before parsing.
- **`DiagnosticCollector`** – aggregates every input/output issue and aborts
  early if anything is wrong.
- **`SchemaUI`** – same runtime used by library consumers; the CLI only wires up
  arguments.

## 3. Input Modes

| Flag                  | Behaviour                                            | Notes                                                                           |
| --------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------- |
| `-s, --schema <SPEC>` | File path, literal JSON/YAML/TOML, or `-` for stdin. | If the path does not exist the CLI treats the argument as inline text.          |
| `-c, --config <SPEC>` | Same semantics as `--schema`.                        | Optional; when omitted, defaults are inferred from the config value if present. |

Constraints enforced by code:

- `stdin` can only be consumed once, so `--schema -` and `--config -` cannot be
  combined.
- If only `--config` is provided, the CLI calls `schema_from_data_value` to
  build a schema with defaults.

## 4. Output & Persistence

- `-o, --output <DEST>` is repeatable; pass `-` to include stdout alongside
  files. Extensions (`.json`, `.yaml`, `.toml`) drive `DocumentFormat`.
- When no destination is set, the CLI writes to `/tmp/schemaui.json` unless
  `--no-temp-file` is passed or `--temp-file <PATH>` overrides the fallback.
- `--no-pretty` toggles compact serialization; pretty output is the default.
- `--force`/`--yes` allows overwriting existing files. Without the flag the CLI
  refuses to run when a destination already exists.

Internally this is powered by `io::output::OutputOptions` so embedding projects
can reuse the exact same serialization logic.

## 5. Argument Reference

| Flag                  | Description                                        | Code hook                           |
| --------------------- | -------------------------------------------------- | ----------------------------------- |
| `-o, --output <DEST>` | Append destinations (`-` writes to stdout).        | `build_output_options`              |
| `--title <TEXT>`      | Overrides the TUI title bar.                       | `SchemaUI::with_title`              |
| `--temp-file <PATH>`  | Custom fallback file when no destinations are set. | `build_output_options`              |
| `--no-temp-file`      | Disable the fallback file behaviour entirely.      | `build_output_options`              |
| `--no-pretty`         | Emit compact JSON/TOML/YAML.                       | `OutputOptions::with_pretty(false)` |
| `--force`, `--yes`    | Allow overwriting files.                           | `ensure_output_paths_available`     |

## 6. Usage Examples

### Schema + config + dual outputs

```bash
schemaui tui \
  --schema ./schema.json \
  --config ./config.yaml \
  -o - \
  -o ./edited.toml
```

### Config only (schema inferred)

```bash
cat defaults.yaml | schemaui --config - --output ./edited.json
```

### Inline schema to avoid double-stdin

```bash
schemaui tui \
  --schema '{"type":"object","properties":{"host":{"type":"string"}}}' \
  --config ./config.json -o -
```

## 7. Diagnostics & Errors

- **Aggregated reporting** – `DiagnosticCollector` stores every input/output
  issue (conflicting stdin, disabled format, existing files) and prints them as
  a numbered list before exiting with a non-zero code.
- **Format inference** – `resolve_format_hint` warns when an extension requires
  a disabled feature (e.g., `.yaml` without the `yaml` feature). The CLI stops
  immediately instead of failing later during serialization.
- **Runtime errors** – everything else bubbles through `color-eyre`, so stack
  traces include context like `failed to parse config as yaml` or
  `failed to compile JSON schema`.

## 8. Library Interop

The CLI is a thin wrapper over `SchemaUI`:

```rust
let mut ui = SchemaUI::new(schema);
if let Some(title) = cli.title.as_ref() {
    ui = ui.with_title(title.clone());
}
if let Some(defaults) = config_value.as_ref() {
    ui = ui.with_default_data(defaults);
}
if let Some(options) = output_settings {
    ui = ui.with_output(options);
}
ui.run()?;
```

This means embedding projects can reproduce the CLI flow verbatim or replace the
front-end entirely (e.g., build a custom CLI or GUI) while reusing the same I/O
and validation pipeline.

## 9. Feature Flags

| Feature          | Effect                                                   |
| ---------------- | -------------------------------------------------------- |
| `json` (default) | Enables JSON parsing/serialization. Always on.           |
| `yaml` (default) | Adds YAML parsing/serialization via `serde_yaml`.        |
| `toml` (opt-in)  | Adds TOML parsing/serialization via `toml`.              |
| `all_formats`    | Convenience feature: enables `json`, `yaml`, and `toml`. |

`DocumentFormat::available_formats()` obeys the same feature matrix, so both the
CLI and host applications automatically reflect build-time capabilities.

## 10. Operational Tips

- Pass one stream as literal text (a non-existent path is treated inline) when
  both schema and config would otherwise require stdin; the executable only
  reads stdin once.
- Prefer explicit extensions for outputs—format inference uses the first file’s
  suffix, and mismatches across files are rejected.
- Combine `-o -` with file outputs to tee the result into CI logs while still
  writing to disk.

## 11. Web Mode

When built with the `web` feature (enabled by default), `schemaui-cli` exposes a
`web` subcommand that proxies the library's browser UI helpers:

```bash
schemaui web \
  --schema ./schema.json \
  --config ./defaults.json \
  --host 127.0.0.1 --port 0 \
  -o -
```

The command reuses the same schema/config pipeline as the TUI flow, then calls
`schemaui::web::session::bind_session` to embed the static assets and APIs from
the crate. The terminal prints the bound address (port `0` selects a random free
port). Hit **Save** to persist edits without leaving, or **Save & Exit** to shut
down the temporary server and emit the resulting JSON through the configured
outputs.

Flags specific to the subcommand:

| Flag            | Description                                 |
| --------------- | ------------------------------------------- |
| `--host <IP>`   | Bind address for the temporary HTTP server. |
| `--port <PORT>` | Bind port (`0` requests an ephemeral port). |

All other arguments (`--schema`, `--config`, `--output`, etc.) behave exactly
like the TUI mode—either inlined or file-backed specs, multiple destinations,
and the same diagnostics.

With these patterns you can script `schemaui` confidently in CI/CD pipelines or
developer tooling.
