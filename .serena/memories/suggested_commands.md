# Suggested Commands

- `cargo fmt` – format all Rust sources before committing changes.
- `cargo test` – run the full workspace test suite (includes `src/tests/**`).
- `cargo clippy --all-targets -- -D warnings` – optional lint sweep to keep CI
  clean.
- `cargo run -p schemaui-cli -- --schema ./schema.json --config ./defaults.yaml -o ./config.toml`
  – exercise the CLI pipeline locally.
- `rg <pattern>` – fast code search (preferred over `grep`).
- `git status -sb` – inspect working tree state on macOS (Darwin).
- `ls`, `sed -n 'start,endp' <file>` – quick file inspection utilities within
  Codex CLI.
