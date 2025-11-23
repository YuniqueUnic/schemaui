# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.4](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.3...schemaui-cli-v0.2.4) - 2025-11-23

### Other

- updated the following local packages: schemaui

## [0.2.2](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.1...schemaui-cli-v0.2.2) - 2025-11-16

### Added

- *(web)* add embedded web UI with `web` feature and CLI subcommand

### Fixed

- *(version)* trigger the release ci/cd.

### Other

- release
- *(ui)* refactor string literals to double quotes and remove save-and-exit feature
- update schemaui version and cli installation instructions

## [0.3.2](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.3.1...schemaui-v0.3.2) - 2025-11-16

### Fixed

- *(gitignore)* exclude node_modules directories recursively

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
