import type {
  JsonValue,
  PreviewResponse,
  RenderContentResult,
  SessionResponse,
  ValidationResponse,
} from "../types";
import type { RenderContentInput, SchemaUiTransport } from "./types";

/**
 * Lazily loads and initializes the `schemaui-wasm` module.
 *
 * Loaded on first use rather than at import time: the wasm binary is a couple
 * of megabytes, and the Playground's paste-a-schema screen renders and is
 * fully usable before anyone needs it. Cached in a module-level promise so a
 * page with several `WasmBackend`s (there is normally exactly one) still only
 * pays the instantiation cost once.
 */
let wasmPromise: Promise<typeof import("@schemaui/wasm")> | null = null;

function loadWasm(): Promise<typeof import("@schemaui/wasm")> {
  wasmPromise ??= import("@schemaui/wasm").then(async (module) => {
    await module.default();
    return module;
  });
  return wasmPromise;
}

/**
 * Parse `text` as JSON, YAML or TOML — whichever it turns out to be.
 *
 * Exposed standalone (not part of {@link SchemaUiTransport}) because it runs
 * *before* there is a schema to build a transport around: the Playground's
 * paste-a-schema screen uses it to accept a schema or a data document in
 * whichever format the visitor pasted or dropped, rather than requiring JSON.
 */
export async function parseDocumentText(text: string): Promise<JsonValue> {
  const wasm = await loadWasm();
  return wasm.parseDocument(text) as JsonValue;
}

/**
 * Runs the schemaui pipeline in-browser against a schema the caller already
 * has in hand — there is no server to bootstrap a session from, so `schema`
 * and `defaults` are supplied up front instead of fetched.
 */
export function createWasmTransport(
  schema: JsonValue,
  defaults: JsonValue,
): SchemaUiTransport {
  return {
    kind: "wasm",

    async bootstrap(): Promise<SessionResponse> {
      const wasm = await loadWasm();
      const built = wasm.buildUiAst(schema, defaults) as {
        ui_ast: SessionResponse["ui_ast"];
        layout: SessionResponse["layout"];
        rich?: SessionResponse["rich"];
        data: JsonValue;
        formats: string[];
      };
      return {
        api_version: "wasm-local",
        // Advertise exactly what this wasm package can do: `renderRich` only
        // exists in packages built with the mermaid feature.
        capabilities:
          typeof wasm.renderRich === "function" ? ["content_render"] : [],
        title: null,
        description: null,
        ui_ast: built.ui_ast,
        layout: built.layout,
        rich: built.rich ?? null,
        data: built.data,
        formats: built.formats,
        expires_in_ms: null,
        draft_restored: false,
      };
    },

    async validate(data: JsonValue): Promise<ValidationResponse> {
      const wasm = await loadWasm();
      return wasm.validate(schema, data) as ValidationResponse;
    },

    async preview(
      data: JsonValue,
      format: string,
      pretty: boolean,
    ): Promise<PreviewResponse> {
      const wasm = await loadWasm();
      return wasm.render(data, format, pretty) as PreviewResponse;
    },

    async renderContent(
      input: RenderContentInput,
    ): Promise<RenderContentResult> {
      const wasm = await loadWasm();
      // The engine's own words for why there is nothing to show: an author
      // error or a package built without the renderer.
      if (typeof wasm.renderRich !== "function") {
        return {
          error: { message: "this build does not include the mermaid renderer" },
        };
      }
      try {
        return (await wasm.renderRich(input.kind, input.source, input.theme)) as RenderContentResult;
      } catch (err) {
        return {
          error: {
            message: typeof err === "string" ? err : err instanceof Error ? err.message : String(err),
          },
        };
      }
    },

    // There is no server to write a draft to; the browser's own localStorage
    // layer (see useSessionActions) is the only durability this mode has.
    async save(): Promise<void> {},

    // There is no server session to end. `useSessionActions.handleExit`
    // checks `kind === "wasm"` and triggers a file download instead — that is
    // what "ending" a Playground session actually means.
    async exit(): Promise<void> {},
  };
}
