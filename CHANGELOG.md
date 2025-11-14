# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- *(web)* bundle the browser UI assets via `include_dir!` and expose
  `schemaui::web::session` (builder, router, server helpers) directly from the
  library when the `web` feature is enabled.
- *(cli)* introduce the `web` subcommand that delegates to the new library API
  instead of hand-rolling the HTTP server, enabling easy customization for
  embedders.
- *(docs)* document the browser workflow in `README.md`, `schemaui-cli/cli_usage.md`,
  and `web/README.md`.

## [0.3.1](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.3.0...schemaui-v0.3.1) - 2025-11-12

### Fixed

- fix build and ci to trigger the latest

### Other

- *(cd)* update upload-rust-binary-action configuration

## [0.2.1](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.0...schemaui-cli-v0.2.1) - 2025-11-12

### Added

- *(config)* add autocorrect and typos configuration files
- *(schemaui)* enhance TUI field rendering and navigation

### Other

- *(readme)* improve formatting and fix typos in README files

## [0.3.0](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.2.0...schemaui-v0.3.0) - 2025-11-12

### Added

- *(config)* add autocorrect and typos configuration files
- *(schemaui)* enhance TUI field rendering and navigation

### Fixed

- *(.github)* rename workflow directory from workflow to workflows
- *(gitignore)* update instructions exclusion pattern

### Other

- *(prek-checks)* remove unnecessary types from pull request trigger
- *(workflows)* add cargo registry token to release workflows
- Fix .gitignore for docs tracking
- *(workflows)* restrict prek checks to main branch only
- *(.gitignore)* update docs directory ignore patterns
- *(readme)* improve formatting and fix typos in README files
- *(runtime)* simplify status message formatting in overlay component
- *(tests)* restructure test modules and update visibility modifiers
- *(input)* refactor keymap handling with KeymapStore
- *(form)* replace FieldValue enum with FieldComponent trait
