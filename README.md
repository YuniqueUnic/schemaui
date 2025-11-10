## schemaui

A library for generating a TUI/Web from json schema for better configuration.

### WIP:

```bash
cargo add schemaui
```

```toml
[dependencies]
schemaui = "0.1.1"
```

1. Core: Using json schema to render a TUI/Web for configuration

References:

1. https://github.com/rjsf-team/react-jsonschema-form
2. https://ui-schema.bemit.codes/examples

More POC:

1. parse json schema at runtime and generate a TUI/Web
2. parse json schema at compile time Then generate the code for TUI/Web, expose
   nessessary APIs for runtime.

### Keyboard Shortcuts

| Context              | Keys                                       | Description                                              |
| -------------------- | ------------------------------------------ | -------------------------------------------------------- |
| Global               | `Tab` / `Shift+Tab`                         | Move between fields and sections                         |
|                      | `Enter`                                     | Open popup/variant selector                              |
|                      | `Ctrl+E`                                    | Edit composite field or open overlay                     |
|                      | `Ctrl+S`                                    | Save (runs schema validation)                            |
|                      | `Ctrl+Q`                                    | Quit (prompts when unsaved)                              |
| Composite/KeyValue   | `Ctrl+N` / `Ctrl+D`                         | Add or remove list/map entries                           |
| lists & key/value    | `Ctrl+←` / `Ctrl+→`                         | Select previous/next entry                               |
| maps                 | `Ctrl+↑` / `Ctrl+↓`                         | Reorder selected entry                                   |
| Overlay editors      | `Ctrl+S` / `Esc`                            | Save overlay / close overlay (press `Esc` twice to abort) |
|                      | `Tab` / `Shift+Tab`                         | Navigate overlay fields                                  |

### License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE) or http://opensource.org/licenses/MIT)

at your option.

### Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
