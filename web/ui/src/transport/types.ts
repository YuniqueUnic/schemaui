import type { JsonValue, PreviewResponse, SessionResponse, ValidationResponse } from "../types";

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
}
