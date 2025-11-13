# Style & Conventions

- Embrace KISS/SOLID: keep modules under ~600 LOC (hard cap 800) and split
  helpers aggressively to avoid god files.
- Form components should mirror shadcn-like composability: favor small, focused
  structs implementing shared traits (e.g., FieldComponent) and keep validators
  close to widgets.
- Keyboard workflow must remain consistent: Tab/Shift-Tab cycling through
  fields/sections, Ctrl+key chords for overlay actions. Update action/status
  bars whenever shortcuts change.
- Documentation-first: update `docs/en/*.md` (progress/todo/structure) whenever
  UX, shortcuts, or pipelines shift so future contributors understand intent.
- Testing strategy: place Rust test modules under `src/tests/**` and expose only
  the minimal `pub(crate)` APIs required for coverage. Each behavior change
  should come with a regression test covering navigation, overlay behavior, or
  schema layout.
