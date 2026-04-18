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

<!-- AUTO-GENERATED:CLI-INSTALL:BEGIN -->

The installed binary is always named `schemaui`, so the normal entry point is
`schemaui -c ./config.json`.

Choose one of the supported channels:

#### Cargo (`cargo install`)

Build from crates.io with Cargo.

```bash
cargo install schemaui-cli
```

#### Cargo binstall

Fetch prebuilt GitHub release binaries through cargo-binstall.

```bash
cargo binstall schemaui-cli
```

#### Homebrew

Install from the repository tap on macOS or Linux.

```bash
brew install YuniqueUnic/schemaui/schemaui
```

#### Scoop

Install on Windows from the repository-hosted Scoop manifest.

```bash
scoop install https://raw.githubusercontent.com/YuniqueUnic/schemaui/main/packaging/scoop/schemaui-cli.json
```

#### Direct download

Download the matching archive from
`https://github.com/YuniqueUnic/schemaui/releases/latest`, extract `schemaui` /
`schemaui.exe`, and place it on your `PATH`.

#### winget manifests

Use the versioned manifests in `packaging/winget` with
`winget install --manifest <dir>`, or submit them upstream to the community
repository.

<!-- AUTO-GENERATED:CLI-INSTALL:END -->

```bash
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

- **`schema_source`** – resolves explicit `--schema`, config-file declarations,
  remote/local schema loading, and final fallback inference in one place.
- **`FormatHint`** – inspects extensions and ensures disabled formats are
  rejected before parsing.
- **`DiagnosticCollector`** – aggregates every input/output issue and aborts
  early if anything is wrong.
- **`SchemaUI`** – same runtime used by library consumers; the CLI only wires up
  arguments.

## 3. Input Modes

| Flag                  | Behaviour                                                                      | Notes                                                                                  |
| --------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| `-s, --schema <SPEC>` | Local path, `file://`, `http(s)://`, literal JSON/YAML/TOML, or `-` for stdin. | Explicit schema wins over any declaration found inside `--config`.                     |
| `-c, --config <SPEC>` | Same loading semantics as `--schema`.                                          | Optional; if `--schema` is omitted the CLI tries config declarations before inferring. |

Constraints enforced by code:

- `stdin` can only be consumed once, so `--schema -` and `--config -` cannot be
  combined.
- Resolution priority is: `--schema` > config declaration > inferred schema.
- Relative local declarations are resolved against the config file's directory.
  For inline/stdin config payloads, relative paths are resolved against the
  current working directory.
- HTTP(S) schema loading is controlled by the `schemaui-cli` `remote-schema`
  feature, which is enabled by default for the CLI. Local paths and `file://`
  URLs always work, and the `schemaui` library crate keeps remote schema loading
  disabled by default. The library crate defaults to `tui + json`, while the CLI
  defaults to the convenience-oriented feature set.
- `json`, `yaml`, and `toml` are all real feature gates. Keep at least one of
  them enabled; disabling all three fails the build with a clear error.

### Config schema auto-detection

When only `--config` is provided, `schemaui-cli` scans the config for a schema
hint before falling back to `schema_from_data_value`.

Supported declarations:

- **JSON**: root `$schema`
- **TOML**: `#:schema https://example.com/schema.json`
- **YAML**: `# yaml-language-server: $schema=...`
- **YAML fallback**: `# @schema ...`

JSON declarations are treated as metadata, so the root `$schema` key is removed
from the in-memory defaults before validation/output.

## 4. Output & Persistence

- `-o, --output <DEST>` is repeatable; pass `-` to include stdout alongside
  files. Extensions (`.json`, `.yaml`, `.toml`) drive `DocumentFormat`.
- When no destination is set, the CLI writes to stdout. Pass
  `--temp-file
  <PATH>` if you explicitly want a fallback file instead.
- `--no-pretty` toggles compact serialization; pretty output is the default.
- `--force`/`--yes` allows overwriting existing files. Without the flag the CLI
  refuses to run when a destination already exists.

Internally this is powered by `io::output::OutputOptions` so embedding projects
can reuse the exact same serialization logic.

## 5. Argument Reference

| Flag                  | Description                                         | Code hook                           |
| --------------------- | --------------------------------------------------- | ----------------------------------- |
| `-o, --output <DEST>` | Append destinations (`-` writes to stdout).         | `build_output_options`              |
| `--title <TEXT>`      | Overrides the TUI title bar.                        | `SchemaUI::with_title`              |
| `--temp-file <PATH>`  | Write to this file when no `--output` is given.     | `build_output_options`              |
| `--no-temp-file`      | Compatibility no-op; stdout is already the default. | `build_output_options`              |
| `--no-pretty`         | Emit compact JSON/TOML/YAML.                        | `OutputOptions::with_pretty(false)` |
| `--force`, `--yes`    | Allow overwriting files.                            | `ensure_output_paths_available`     |

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

### Config only with schema declaration

```bash
schemaui web --config ./config.yaml
```

```yaml
# yaml-language-server: $schema=./schema.json
name: api
port: 8080
```

### Explicit schema override beats the file header

```bash
schemaui \
  --schema https://example.com/runtime.schema.json \
  --config ./config.toml
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

| Feature          | Effect                                                     |
| ---------------- | ---------------------------------------------------------- |
| `json` (default) | Enables JSON parsing/serialization and JSON format probes. |
| `yaml`           | Adds YAML parsing/serialization via `serde_yaml`.          |
| `toml` (opt-in)  | Adds TOML parsing/serialization via `toml`.                |
| `all_formats`    | Convenience feature: enables `json`, `yaml`, and `toml`.   |

`DocumentFormat::available_formats()` obeys the same feature matrix, so both the
CLI and host applications automatically reflect build-time capabilities. At
least one of `json`, `yaml`, or `toml` must remain enabled.

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
