# Project Overview

- **Purpose**: `schemaui` is a Rust crate plus CLI that converts JSON Schema
  documents into interactive terminal user interfaces (TUIs). It parses schemas
  into a navigable form tree, renders them with ratatui/crossterm, and
  continuously validates user edits so the resulting JSON stays correct before
  saving.
- **High-Level Architecture**: Input/output helpers live under `io::*`, schema
  parsing/refinement is in `schema::*` (loader/resolver/layout), form state +
  reducers are under `form::*`, runtime orchestration (keymaps, overlays,
  status) is in `app::*`, and all ratatui drawing code sits in
  `presentation::*`. CLI wrapper (`schemaui-cli`) reuses the library pipeline
  but wires clap diagnostics and multi-destination outputs.
- **Key UX Concepts**: Root sections are derived from top-level schema objects,
  nested objects collapse into child sections, and complex nodes (arrays,
  key/value maps, composites) open overlays for focused editing. Navigation is
  keyboard-first (Tab/Shift-Tab, Ctrl modifiers) with an action bar + status bar
  summarizing hints.
