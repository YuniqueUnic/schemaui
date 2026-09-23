import type {
  JsonValue,
  PreviewResponse,
  SessionResponse,
  ValidationResponse,
} from "./types";
import type { RenderContentInput, RenderContentResult } from "./transport/types";

async function request<T>(
  path: string,
  options?: RequestInit & { json?: unknown },
): Promise<T> {
  const init: RequestInit = {
    headers: {
      "Content-Type": "application/json",
      ...(options?.headers ?? {}),
    },
    ...options,
  };

  if (options?.json !== undefined) {
    init.body = JSON.stringify(options.json);
  }

  const response = await fetch(path, init);
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `Request failed: ${response.status}`);
  }
  if (response.status === 204) {
    return {} as T;
  }
  return (await response.json()) as T;
}

export function fetchSession(): Promise<SessionResponse> {
  return request<SessionResponse>("/api/v1/session");
}

export function validateData(data: JsonValue): Promise<ValidationResponse> {
  return request<ValidationResponse>("/api/v1/validate", {
    method: "POST",
    json: { data },
  });
}

export function renderPreview(
  data: JsonValue,
  format: string,
  pretty: boolean,
): Promise<PreviewResponse> {
  return request<PreviewResponse>("/api/v1/preview", {
    method: "POST",
    json: { data, format, pretty },
  });
}

/** Write a durable draft that outlives the process (requires `Draft` capability). */
export function persistData(data: JsonValue) {
  return request("/api/v1/save", {
    method: "POST",
    json: { data },
  });
}

export function exitSession(data: JsonValue, commit: boolean) {
  return request("/api/v1/exit", {
    method: "POST",
    json: { data, commit },
  });
}

/** Fetch the raw JSON Schema that backs this session (`/api/v1/schema`). */
export function fetchSchema(): Promise<JsonValue> {
  return request<JsonValue>("/api/v1/schema");
}

/**
 * Render one Mermaid diagram (`POST /api/v1/render`). Unlike the other
 * helpers, a rejection here is *data* — an author error shown next to their
 * source — so 4xx bodies are returned as `{ error }`, not thrown.
 */
export async function renderDiagram(
  input: RenderContentInput,
): Promise<RenderContentResult> {
  const response = await fetch("/api/v1/render", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
  const body = (await response.json()) as RenderContentResult;
  if (!response.ok && !("error" in body)) {
    return { error: { message: `Request failed: ${response.status}` } };
  }
  return body;
}
