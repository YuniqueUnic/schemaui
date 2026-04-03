# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.2](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.4.1...schemaui-cli-v0.4.2) - 2026-04-03

### Other

- updated the following local packages: schemaui

## [0.4.1](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.4.0...schemaui-cli-v0.4.1) - 2026-04-02

### Other

- updated the following local packages: schemaui

## [0.4.0](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.3.0...schemaui-cli-v0.4.0) - 2026-04-02

### Fixed

- *(cli)* add explicit subcommands and default to TUI mode

## [0.3.0](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.8...schemaui-cli-v0.3.0) - 2026-04-01

### Added

- add support for recursive schema references in TUI generation
- add UiAstBundle for sharing compiled UI artifacts between frontends
- *(cli)* add TUI snapshot generation command
- *(cli)* add web-snapshot command to precompute session snapshots

### Other

- [**breaking**] rename precompile terminology to artifact across codebase
- *(precompile)* rename PrecompiledUiBundle to UiArtifactBundle and add TuiArtifacts
- *(tui)* add layout navigation support for TUI forms

## [0.2.8](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.7...schemaui-cli-v0.2.8) - 2026-03-28

### Added

- *(schema)* preserve reference annotations during resolution

## [0.2.7](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.6...schemaui-cli-v0.2.7) - 2025-12-03

### Added

- *(schemaui-cli)* add release profile optimizations

## [0.2.6](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.5...schemaui-cli-v0.2.6) - 2025-12-03

### Other

- updated the following local packages: schemaui
# schemaui-cli Changelog

All notable changes to the CLI crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.4](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.3...schemaui-cli-v0.2.4) - 2025-11-23

### Other

- updated the following local packages: schemaui

## [0.2.2](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.1...schemaui-cli-v0.2.2) - 2025-11-16

### Added

- _(web)_ add embedded web UI with `web` feature and CLI subcommand

### Fixed

- _(version)_ trigger the release ci/cd.

### Other

- release
- _(ui)_ refactor string literals to double quotes and remove save-and-exit
  feature
- update schemaui version and cli installation instructions

## [0.2.1](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.0...schemaui-cli-v0.2.1) - 2025-11-12

### Added

- _(config)_ add autocorrect and typos configuration files
- _(schemaui)_ enhance TUI field rendering and navigation

### Other

- _(readme)_ improve formatting and fix typos in README files
