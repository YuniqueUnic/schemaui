# Custom frontend example

A single `index.html` — no build step, no framework, no dependency — that talks
to the versioned `/api/v1/*` contract directly. It exists to prove that contract
is real: any stack (or no stack) can drive a `schemaui` web session.

Run it against any schema:

```sh
schemaui web --schema examples/simple.schema.json --frontend examples/frontend
```

Then open the printed URL. The page fetches `GET /api/v1/session`, renders the
schema's top-level scalar fields (string, integer, number, boolean, enum), and
wires them to `POST /api/v1/validate` and `POST /api/v1/preview` on every
change, plus `POST /api/v1/save` and `POST /api/v1/exit` on the two buttons.

**Scope.** Array, composite (`oneOf`/`anyOf`) and key-value map fields render as
"unsupported" placeholders rather than being reimplemented — that machinery
already exists in the bundled React UI (`web/ui/`), and duplicating it here
would just give the two a chance to drift. This example's job is to demonstrate
the wire format, not to replace the default frontend.
