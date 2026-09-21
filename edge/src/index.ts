/**
 * A stateless HTTP API over the `schemaui-wasm` exports.
 *
 * "Stateless" is the point: schema in, UiAst / validation / rendered
 * document out, nothing kept between requests — no session, no storage. A
 * caller that wants a session (save, resume, timeout) runs the real HTTP
 * server (`schemaui web`) or the Playground instead; this exists for callers
 * that want neither a Rust dependency nor a wasm loader of their own. See
 * `docs/en/decisions/0005-wasm-core-and-hosting.md`.
 *
 * Each route is named after the wasm export it calls and takes exactly that
 * export's arguments as a JSON body — a thin proxy, not a second contract to
 * keep in sync with the first.
 */
import wasmModule from "../../schemaui-wasm/pkg/schemaui_wasm_bg.wasm";
import * as wasm from "../../schemaui-wasm/pkg/schemaui_wasm.js";

wasm.initSync({ module: wasmModule });

const CORS_HEADERS: Record<string, string> = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type",
};

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...CORS_HEADERS },
  });
}

/** wasm-bindgen turns a Rust `Result::Err` into a thrown JS *string* (see
 * `schemaui-wasm/src/lib.rs`), not an `Error` — handle both. */
function errorResponse(err: unknown): Response {
  const message = typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
  return json({ error: message }, 400);
}

/** One route per wasm export, each just unpacking its JSON body into the
 * export's positional arguments. */
const ROUTES: Record<string, (body: Record<string, unknown>) => unknown> = {
  "/buildUiAst": (body) => wasm.buildUiAst(body.schema, body.defaults ?? {}),
  "/validate": (body) => wasm.validate(body.schema, body.data),
  "/render": (body) => wasm.render(body.data, body.format as string, Boolean(body.pretty)),
  "/schemaWithDefaults": (body) => wasm.schemaWithDefaults(body.schema, body.data),
  "/schemaFromData": (body) => wasm.schemaFromData(body.data),
  "/parseDocument": (body) => wasm.parseDocument(body.text as string),
};

const INDEX_BODY = {
  name: "schemaui-edge",
  description: "Stateless HTTP API over the schemaui-wasm exports.",
  routes: Object.keys(ROUTES).map((path) => `POST ${path}`),
};

export default {
  async fetch(request: Request): Promise<Response> {
    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: CORS_HEADERS });
    }

    const { pathname } = new URL(request.url);

    if (request.method === "GET" && pathname === "/") {
      return json(INDEX_BODY);
    }

    const handler = ROUTES[pathname];
    if (!handler) {
      return json({ error: `no such route: ${pathname}` }, 404);
    }
    if (request.method !== "POST") {
      return json({ error: "expected POST" }, 405);
    }

    let body: Record<string, unknown>;
    try {
      body = await request.json();
    } catch (err) {
      return errorResponse(err);
    }

    try {
      return json(handler(body));
    } catch (err) {
      return errorResponse(err);
    }
  },
};
