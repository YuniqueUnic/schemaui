WIP:

```bash
cargo add schemaui
```

```toml
[dependencies]
schemaui = "0.1.0"
```

1. Core: Using json schema to render a TUI/Web for configuration

References:

1. https://github.com/rjsf-team/react-jsonschema-form
2. https://ui-schema.bemit.codes/examples

More POC:

1. parse json schema at runtime and generate a TUI/Web
2. parse json schema at compile time Then generate the code for TUI/Web, expose
   nessessary APIs for runtime.
