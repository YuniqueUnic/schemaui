# schemaui-edge

A stateless HTTP API over the `schemaui-wasm` exports, deployed as a Cloudflare
Worker. Schema in, UiAst / validation / rendered document out — no session, no
storage. See
[`docs/en/decisions/0005-wasm-core-and-hosting.md`](../docs/en/decisions/0005-wasm-core-and-hosting.md)
§7 for why this exists and what it deliberately does not do.

## Routes

One per `schemaui-wasm` export, named after it. All are `POST` with a JSON body;
the body's keys are that export's arguments.

| Route                 | Body                       |
| --------------------- | -------------------------- |
| `/buildUiAst`         | `{ schema, defaults? }`    |
| `/validate`           | `{ schema, data }`         |
| `/render`             | `{ data, format, pretty }` |
| `/schemaWithDefaults` | `{ schema, data }`         |
| `/schemaFromData`     | `{ data }`                 |
| `/parseDocument`      | `{ text }`                 |

`GET /` lists the routes. Errors come back as `{ "error": "..." }` with a 4xx
status.

## Local development

```sh
just dev-edge      # builds schemaui-wasm, then `wrangler dev`
```

or manually:

```sh
just build-wasm
cd edge
pnpm install
pnpm dev
```

`wrangler dev --local` needs no Cloudflare account.

## Deploying

```sh
just deploy-edge
```

Needs `wrangler login` (interactive) or `CLOUDFLARE_API_TOKEN` +
`CLOUDFLARE_ACCOUNT_ID` in the environment — the same two variables the
`Deploy Edge API` GitHub Actions workflow expects as repository secrets.
