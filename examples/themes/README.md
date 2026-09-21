# `--web-theme` example

`midnight.css` overrides a handful of the design tokens documented in
`web/ui/src/styles/globals.css` (the Tailwind v4 `@theme` block). Point
`--web-theme` at it, or at a copy with your own values:

```sh
schemaui web --schema examples/simple.schema.json --web-theme examples/themes/midnight.css
```

Check it took effect two ways: `GET /api/v1/theme.css` on the running session
should return the file verbatim, and `capabilities` in `GET /api/v1/session`
should include `"theme"`.

There is deliberately no second bundled theme beyond this one small example —
see `docs/en/decisions/0003-user-css-theming.md` for why.
