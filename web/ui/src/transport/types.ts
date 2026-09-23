import type {
  JsonValue,
  PreviewResponse,
  RenderedSvg,
  SessionResponse,
  ValidationResponse,
} from "../types";

/** One on-demand render request. Only Mermaid is renderable this way. */
export interface RenderContentInput {
  kind: "mermaid";
  source: string;
  theme: "light" | "dark";
}

/**
 * Either the rendered SVG, or the renderer's message as data — an author
 * error shown next to the source, not an exception.
 */
export type RenderContentResult =
  | { svg: RenderedSvg }
  | { error: { message: string } };

/**
 * Everything the form needs from wherever the schema pipeline actually runs.
 *
 * Two implementations: `httpTransport` talks to a running `schemaui` server;
 * `createWasmTransport` runs the same pipeline in-browser via the
 * `schemaui-wasm` package. `App` and `useSessionActions` are written against
 * this interface, not against either backend, which is what lets the
 * Playground reuse the whole form instead of forking it — see
 * `docs/en/decisions/0005-wasm-core-and-hosting.md`.
 */
export interface SchemaUiTransport {
  /** Which backend this is, for the few places behavior legitimately differs
   * (see `useSessionActions.handleExit`) rather than being papered over. */
  kind: "http" | "wasm";
  bootstrap(): Promise<SessionResponse>;
  validate(data: JsonValue): Promise<ValidationResponse>;
  preview(data: JsonValue, format: string, pretty: boolean): Promise<PreviewResponse>;
  /** Persist a draft. A no-op where there is nowhere durable to write it. */
  save(data: JsonValue): Promise<void>;
  /** End the session. A no-op where there is no server session to end. */
  exit(data: JsonValue, commit: boolean): Promise<void>;
  /**
   * Render a diagram on demand, when the backend has the renderer at all.
   * Optional rather than always-present: its absence — like a session whose
   * `capabilities` lack `content_render` — is the signal to fall back to a
   * plain text area instead of mounting a live preview.
   */
  renderContent?(input: RenderContentInput): Promise<RenderContentResult>;
}
