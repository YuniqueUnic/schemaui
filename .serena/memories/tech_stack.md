# Tech Stack

- **Language & Toolchain**: Rust 2021 edition, built via Cargo; auxiliary CLI
  (`schemaui-cli`) lives in the same workspace.
- **Core Crates**: `serde` / `serde_json` / `serde_yaml` / `toml` for IO,
  `schemars` + `jsonschema` for schema typing/validation, `ratatui` for
  rendering, `crossterm` for terminal events, `indexmap` for order-preserving
  traversal, `once_cell` for lazy keymaps, `clap` + `color-eyre` for CLI UX.
- **Testing/Formatting**: `cargo fmt` with rustfmt, `cargo test` for
  unit/integration coverage (tests live under `src/tests/**` after recent
  migration). Optional `cargo clippy --all-targets -- -D warnings` to enforce
  lints.
- **OS Context**: Running on macOS (Darwin); standard BSD userland utilities
  (`ls`, `sed`, `rg`, `git`) are available through the Codex CLI harness.
