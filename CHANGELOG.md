# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> CLI-specific release notes now live under `schemaui-cli/CHANGELOG.md`.

## [0.6.0](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.5.0...schemaui-v0.6.0) - 2026-04-01

### Added

- preserve field and root declaration order in form schemas
- enhance StatusBar component with improved status handling and UI updates
- *(tui)* add comprehensive footer component with enhanced styling
- *(tui)* implement paginated help overlay with shortcut scrolling
- add support for recursive schema references in TUI generation
- *(precompile)* add PrecompiledUiBundle w/ fingerprint and TUI artifacts
- add UiAstBundle for sharing compiled UI artifacts between frontends
- *(cli)* add TUI snapshot generation command
- *(ui-ast)* add defaults and layout modules with tests
- *(web)* add compile-time session snapshot generation and embedding support
- *(tui)* add compile-time TUI form schema generation and precompilation support
- *(compile-time)* add compile-time UI AST generation and integration

### Other

- *(readme)* fix line wrapping in Chinese README documentation
- *(tui)* add invalid composite overlay test case
- *(schema)* [**breaking**] replace direct schema layout with UI AST pipeline
- [**breaking**] rename precompile terminology to artifact across codebase
- *(precompile)* rename PrecompiledUiBundle to UiArtifactBundle and add TuiArtifacts
- *(web)* add Deserialize derive to SessionResponse
- *(examples)* add precompile codegen and TUI examples
- *(precompile)* rename `compile_time` feature and module to `precompile`
- *(schema)* add compile-time TUI tests and refactor required field handling
- *(tui)* add layout section description to overlays
- *(tui)* add layout navigation support for TUI forms
- *(tui)* implement layout navigation support in form state
- *(web)* add UI layout generation and include in session response
- *(ui-ast)* add pointer indexing functionality
- *(web/ui)* update dependencies to latest versions
- *(web)* update build command to use embedded target

## [0.5.0](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.4.3...schemaui-v0.5.0) - 2026-03-28

### Added

- *(schema)* preserve reference annotations during resolution

### Other

- update deps

## [0.4.3](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.4.2...schemaui-v0.4.3) - 2025-12-03

### Added

- *(schemaui-cli)* add release profile optimizations

## [0.4.2](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.4.1...schemaui-v0.4.2) - 2025-12-03

### Added

- *(web)* Implement new exit behavior based on different conditions

### Fixed

- *(web)* fix preview language highlight

### Other

- *(web)* switch to embedded build mode for web UI compilation
- bump schemaui version from 0.4.0 to 0.4.1 in README examples
- *(web)* refactor app.tsx to clarify the state and application effect
- *(web)* update compiled CSS bundle with latest Tailwind styles
- *(web)* remove outdated compiled CSS bundle
- *(web)* update compiled CSS bundle with latest styles
- *(web)* refactor NodeRenderer with component contract pattern

## [0.4.0](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.3.3...schemaui-v0.4.0) - 2025-11-23

### Added

- *(tui)* add help overlay with keyboard shortcuts and validation errors, rename ui to backend in demo
- *(ui)* add variant selector popup for composite list entries with multiple variants

### Fixed

- *(tui)* prevent auto-creating new entry when deleting last item from composite list in overlay
- *(tui)* refactor focus navigation logic in form state
- *(tests)* enhance route schema and remove outdated overlay tests
- *(composite)* change anyOf behavior to return single value instead of array

### Other

- *(web)* improve array field editing with overlay support and fix composite list rendering
- *(deps)* upgrade jsonschema from 0.33.0 to 0.37 and update API usage
- *(readme)* replace demo GIF with asciinema recording and add animated signature header
- *(docs)* remove outdated fix report and update gitignore for architecture documentation
- *(tui/view)* fix rendering order to ensure popup always appears on top of overlay
- *(tui)* implement auto-save on exit and fix overlay save propagation to root form state
- *(tui)* add unit tests for FieldSchema display_label behavior and improve composite_list assertion clarity
- *(tui)* fix popup width calculation and improve composite list multi-choice handling for arrays
- *(ui_ast/tui)* promote plain object arrays to single-variant composites and improve variant title generation
- *(workflows)* migrate from npm to pnpm for web UI builds across all GitHub Actions workflows
- *(tui/app/overlay)* extract popup selection handling logic into dedicated popup_ops submodule
- *(tui/app/overlay)* extract overlay core and opening logic into dedicated submodules
- *(tui/app)* extract overlay input handling and list operations into dedicated submodules
- *(tui/app)* split overlay app module into focused submodules and extract validation logic
- *(tui/app)* extract overlay validation logic and add scalar array overlay test
- *(tui)* remove unused overlay code and clean up imports
- *(tui/app)* extract overlay management logic from runtime module into dedicated overlay/app submodule
- *(tui/app)* extract overlay editor logic into separate state and editor modules
- *(tests)* consolidate test modules under tui directory to match new module structure
- *(readme)* update architecture diagrams and API documentation to reflect tui module consolidation
- *(state)* remove form module and consolidate all state imports to tui::state
- *(form)* remove legacy form/ui module and migrate all UI store code to tui::state::ui_store
- *(lib)* remove app and presentation compatibility shims and migrate all imports to tui module
- *(domain)* remove domain module and migrate all imports to tui::model
- *(tui)* consolidate form state imports to use tui::state module
- *(tui/state)* consolidate domain imports to use tui::model module
- *(tui)* consolidate app and presentation modules into unified tui structure
- *(state)* migrate form state modules from src/form to src/tui/state
- *(schema)* remove domain/schema.rs and migrate form schema building to tui/model module
- *(core)* simplify frontend execution API and remove temporary re-exports
- *(core)* extract shared pipeline and introduce pluggable Frontend trait
- *(tui)* extract TUI session logic into dedicated module
- *(cli)* unify TUI and web frontends under common execution path with enhanced CLI options
- *(web)* rebuild web UI bundle with updated React dependencies
- *(web)* rebuild web UI bundle with updated React dependencies
- *(web)* rebuild web UI bundle with updated React dependencies
- *(web)* rebuild web UI bundle with updated React dependencies
- *(dev)* add development guide and ignore log files
- *(form)* remove unused section_id field from test field schemas
- *(schema)* remove unused section_id field from FieldSchema and improve runtime configuration
- *(ui)* improve variant switching logic for oneOf/anyOf schemas
- *(web)* rebuild web UI bundle with updated React dependencies
- *(web)* rebuild web UI bundle with updated React dependencies
- *(web)* rebuild web UI bundle with updated React dependencies
- *(cli)* introduce modular CLI structure with TUI and web support
- *(composite)* enhance variant selector UI with descriptions and improved layout
- *(web)* rebuild web UI bundle with updated React dependencies
- *(web)* improve UI styling and remove redundant error toasts
- *(web)* rebuild web UI bundle with updated dependencies
- *(web)* rebuild web UI bundle with updated React dependencies

## [0.3.2](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.3.1...schemaui-v0.3.2) - 2025-11-16

### Fixed

- *(gitignore)* exclude node_modules directories recursively

## [0.3.1](https://github.com/YuniqueUnic/schemaui/compare/schemaui-v0.3.0...schemaui-v0.3.1) - 2025-11-12

### Fixed

- fix build and ci to trigger the latest

### Other

- *(cd)* update upload-rust-binary-action configuration

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
