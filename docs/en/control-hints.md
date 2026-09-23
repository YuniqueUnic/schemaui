# Presentation Hints: Choosing the Control from the Schema

A JSON Schema says what a value _is_; it does not say how to _edit_ it. These
`x-` keywords close that gap. They never affect validation — a schema with hints
validates exactly like the same schema without them.

All hints are optional. A field with no hint gets the control that fits its
shape, so a schema written for another tool still renders a usable form.

A runnable schema exercising every value below — including the marked sliders,
the stepped variants, and the refused-hint fallbacks — lives in
[`examples/controls-gallery.schema.json`](../../examples/controls-gallery.schema.json);
serve it with `schemaui web --schema examples/controls-gallery.schema.json`. The
screenshots in the README were captured from it.

## `x-control`

Names the control to use for a node.

```jsonc
{
  "type": "object",
  "properties": {
    "retention_days": {
      "type": "integer",
      "minimum": 1,
      "maximum": 365,
      "x-control": "slider"
    }
  }
}
```

| Value       | Renders                                  | Requires                    |
| ----------- | ---------------------------------------- | --------------------------- |
| `text`      | Single-line text input                   | `string`                    |
| `textarea`  | Multi-line text area                     | `string`                    |
| `select`    | Dropdown                                 | `enum`                      |
| `segmented` | Inline segmented buttons                 | `enum`                      |
| `radio`     | Radio group                              | `enum`                      |
| `switch`    | Toggle                                   | `boolean`                   |
| `checkbox`  | Checkbox                                 | `boolean`                   |
| `slider`    | Single-thumb slider                      | `number`/`integer` + bounds |
| `range`     | Two-thumb range slider                   | array of 2 numbers + bounds |
| `color`     | Colour picker                            | `string`                    |
| `mermaid`   | Source editor with live rendered preview | `string`                    |

A `range` field is an array, and it must declare both ends so the UI knows how
many thumbs to draw:

```jsonc
{
  "type": "array",
  "items": { "type": "integer" },
  "minItems": 2,
  "maxItems": 2,
  "minimum": 0,
  "maximum": 100,
  "x-control": "range"
}
```

### When a hint cannot be honoured

A control that does not fit its value is a schema bug, and schemaui stops at
load time rather than guessing:

```text
`x-control: "slider"` on `/name` needs a number, but the property is a string
```

An unknown control name fails the same way. A typo such as `"silder"` is
rejected instead of quietly falling back to a text box, because a silent
fallback looks exactly like the hint being ignored.

The one case that does _not_ fail is a control this build has never heard of
arriving through a precompiled snapshot: the frontend falls back to the default
for the node's shape, so an older UI still renders a newer schema.

## Bounds: `minimum`, `maximum`, `multipleOf`

Sliders and ranges read their track from the ordinary validation keywords — no
extra hint is needed. `multipleOf` becomes the slider step.

These keywords are read for **every** numeric field, not only sliders, so a
plain number input can still show the valid range. Declaring them does _not_
turn a field into a slider; only `x-control: "slider"` does.

If `multipleOf` is absent, the step is derived: integers step by `1`, and a
`number` gets a step that divides the span into roughly a hundred stops, snapped
to a round magnitude. So `0`–`1` steps by `0.01` and `0`–`100` by `1` — a fixed
step of `1` would leave a unit interval with only two reachable positions.

A slider needs **both** ends. With only one, the other would have to be
invented, and an invented maximum is a worse answer than a number box — so the
field falls back to a plain input.

## `x-slider-marks`

Labelled stops drawn under a slider track.

```jsonc
{
  "type": "integer",
  "minimum": 0,
  "maximum": 100,
  "x-control": "slider",
  "x-slider-marks": [0, 50, 100]
}
```

A mark is either a bare number or an object when it needs a label:

```jsonc
"x-slider-marks": [
  { "value": 0, "label": "Off" },
  { "value": 50, "label": "Half" },
  { "value": 100, "label": "Full" }
]
```

## `x-multiline`

Renders a string as a text area. Equivalent to `x-control: "textarea"`; both are
supported because this one predates the general hint.

```jsonc
{ "type": "string", "x-multiline": true }
```

## `x-content`

Attaches a figure or prose block to a node itself — a diagram next to the
control, not inside any value. The AST carries only the source; the engine
renders and sanitises it at session build time and ships the result in
`SessionResponse.rich`, addressed by content id.

```jsonc
{
  "type": "object",
  "properties": {
    "region": {
      "type": "string",
      "enum": ["eu", "us"],
      "x-content": {
        "type": "mermaid",
        "source": "flowchart LR; You-->CDN-->Edge"
      }
    }
  }
}
```

`type` is `mermaid` (rendered diagram, light and dark variants), `svg` (raw SVG,
rebuilt through a whitelist — `<script>`, event handlers and `foreignObject`
never survive), or `markdown` (converted to sanitised HTML). A frontend without
the rendered asset falls back to showing the source as code; that is a
capability fallback, not an error. An unparsable diagram is an error and is
shown next to the source it came from. A runnable schema exercising every
rich-content shape lives in
[`examples/rich-content.schema.json`](../../examples/rich-content.schema.json);
serve it with `schemaui web --schema examples/rich-content.schema.json`.

## `x-options`

Per-option detail for an enum, aligned with the enum values by index. Each entry
may carry a `label` (overriding the derived one, including in the terminal UI),
a `description`, and a `content` figure rendered beside the option.

```jsonc
{
  "type": "string",
  "enum": ["mesh", "star", "ring"],
  "x-control": "radio",
  "x-options": [
    {
      "label": "Mesh",
      "description": "Every node connected to every other",
      "content": {
        "type": "mermaid",
        "source": "flowchart LR; A---B & C; B---C"
      }
    },
    { "label": "Star" },
    {}
  ]
}
```

The list must line up with the enum one-for-one, and entries may be empty.
`segmented` has no room for figures, so `x-options` content on a segmented
control is rejected at load time — use `radio` or `select`.

## `x-visible-when`

Hides a node until a sibling property matches. The sibling is named by its
**property key**, not its pointer, and must be a property of the same object.

```jsonc
{
  "type": "object",
  "properties": {
    "advanced": { "type": "boolean", "default": false },
    "retries": {
      "type": "integer",
      "x-visible-when": { "field": "advanced", "op": "equals", "value": true }
    }
  }
}
```

| Key     | Meaning                      |
| ------- | ---------------------------- |
| `field` | Sibling property key to test |
| `op`    | `equals` or `contains`       |
| `value` | Value to compare against     |

A rule pointing at a property the object does not declare is rejected at load
time. Such a typo would hide the node forever, with nothing on screen to explain
why it never appeared.

## Precedence

When both are present, `x-control` wins:

1. the requested control, if it fits the value;
2. otherwise the default for the node's shape — `select` for an `enum`, `switch`
   for a `boolean`, `textarea` for a multi-line string, `text` for other
   strings, a plain numeric input for numbers;
3. otherwise a plain input.

## Where hints are parsed

`src/ui_ast/builder/hints.rs` owns the vocabulary and the validation. Every node
is assembled through `hints::node`, so a hint cannot be dropped by one
construction path while working in another. The parsed values land on `UiNode`
as `control` and `bounds`; see [`ui-ast-design.md`](./ui-ast-design.md) for the
surrounding structure.
