import type {
  JsonValue,
  PreviewResponse,
  SessionResponse,
  ValidationResponse,
} from './types';

async function request<T>(
  path: string,
  options?: RequestInit & { json?: unknown },
): Promise<T> {
  const init: RequestInit = {
    headers: {
      'Content-Type': 'application/json',
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
  return request<SessionResponse>('/api/session');
}

export function validateData(data: JsonValue): Promise<ValidationResponse> {
  return request<ValidationResponse>('/api/validate', {
    method: 'POST',
    json: { data },
  });
}

export function renderPreview(
  data: JsonValue,
  format: string,
  pretty: boolean,
): Promise<PreviewResponse> {
  return request<PreviewResponse>('/api/preview', {
    method: 'POST',
    json: { data, format, pretty },
  });
}

export function persistData(data: JsonValue) {
  return request('/api/save', {
    method: 'POST',
    json: { data },
  });
}

export function exitSession(data: JsonValue, commit: boolean) {
  return request('/api/exit', {
    method: 'POST',
    json: { data, commit },
  });
}
