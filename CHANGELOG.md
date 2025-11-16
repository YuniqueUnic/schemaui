# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2](https://github.com/YuniqueUnic/schemaui/compare/schemaui-cli-v0.2.1...schemaui-cli-v0.2.2) - 2025-11-16

### Added

- *(web)* add embedded web UI with `web` feature and CLI subcommand

### Other

- *(ui)* refactor string literals to double quotes and remove save-and-exit feature
- update schemaui version and cli installation instructions

## [0.3.2](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.3.1...schemaui-v0.3.2) - 2025-11-16

### Added

- *(web)* add session export endpoint and dumping functionality
- *(web/ui)* initialize web UI project structure
- *(web)* add embedded web UI with `web` feature and CLI subcommand
- *(form)* introduce view module for UI state management
- *(form)* introduce UI stores for managing navigation state
- *(tui)* refactor TUI layout and improve component design
- *(palette)* introduce component palette for UI customization
- *(overlay)* implement entry tab navigation and focus management
- *(overlay)* implement nested overlay support with stack-based navigation

### Fixed

- *(ui)* enhance breadcrumb generation with section tree context
- *(ui)* add support for dynamic language syntax highlighting
- *(ui)* add syntax highlighting with Prism.js for preview pane
- *(web)* implement light/dark theme support across UI components

### Other

- *(release)* add Node.js setup and web UI build steps
- *(ui)* implement theme-aware styling with CSS variables
- *(ui)* implement keyboard shortcut for saving session
- *(parser)* replace JSON schema parser with UI AST pipeline
- *(ui)* introduce UI AST module and refactor schema processing
- *(ui)* implement overlay provider and enhance node rendering
- *(web)* introduce UI AST generation from JSON schema
- *(ui)* introduce UI AST and refactor editor components
- *(web/ui)* implement composite array editor and enhance variant handling
- *(web)* implement composite field editor with variant selection
- *(web)* implement exit guard dialog and validation handling
- *(ui)* refactor string literals to double quotes and remove save-and-exit feature
- *(web)* add TypeScript type generation and integrate web UI build process
- *(web)* introduce flexible asset loading with WebAssetProvider
- *(web)* implement JSON schema-based web UI session with validation and preview
- *(runtime)* add support for entry label in overlay state
- *(runtime)* add overlay validator caching and improve focus logic
- *(runtime)* encapsulate overlay fields and improve entry tab logic
- *(form)* implement composite list popup and selection handling
- *(tui)* refactor TUI layout and improve component design
- *(runtime)* enhance composite field handling with improved variant selection
- *(examples)* add complex and nested JSON schema examples
- *(overlay)* optimize overlay context handling and exit logic
- *(fields)* streamline text wrapping and error line rendering
- *(composite)* enhance value matching logic and add tests
- *(runtime)* implement overlay entry navigation and selection logic
- *(overlay)* add focus mode reset logic and improve tab navigation
- *(presentation)* extract tab strip rendering into dedicated module
- *(presentation)* implement dynamic tab scrolling and improve field rendering
- update schemaui version and cli installation instructions

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
