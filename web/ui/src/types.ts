import type { FieldError as ServerFieldError } from '@schemaui/types/FieldError';
import type { PreviewRequest as ServerPreviewRequest } from '@schemaui/types/PreviewRequest';
import type { PreviewResponse as ServerPreviewResponse } from '@schemaui/types/PreviewResponse';
import type { SaveRequest as ServerSaveRequest } from '@schemaui/types/SaveRequest';
import type { SessionResponse as ServerSessionResponse } from '@schemaui/types/SessionResponse';
import type { ExitRequest as ServerExitRequest } from '@schemaui/types/ExitRequest';
import type { ValidateRequest as ServerValidateRequest } from '@schemaui/types/ValidateRequest';
import type { ValidationResponse as ServerValidationResponse } from '@schemaui/types/ValidationResponse';

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export type FieldError = ServerFieldError;

export type ValidationResponse = Omit<ServerValidationResponse, 'errors'> & {
  errors: FieldError[];
};

export type SessionResponse = Omit<ServerSessionResponse, 'data' | 'ui_ast'> & {
  data: JsonValue;
  ui_ast: UiAst;
};

export type SaveRequest = Omit<ServerSaveRequest, 'data'> & {
  data: JsonValue;
};

export type ExitRequest = Omit<ServerExitRequest, 'data'> & {
  data: JsonValue;
};

export type ValidateRequest = Omit<ServerValidateRequest, 'data'> & {
  data: JsonValue;
};

export type PreviewRequest = Omit<ServerPreviewRequest, 'data'> & {
  data: JsonValue;
};

export type PreviewResponse = ServerPreviewResponse;

export type ScalarKind = 'string' | 'integer' | 'number' | 'boolean';
export type CompositeMode = 'one_of' | 'any_of';

export type UiNodeKind =
  | { type: 'field'; scalar: ScalarKind; enum_options?: string[] | null }
  | { type: 'array'; item: UiNodeKind; min_items?: number | null; max_items?: number | null }
  | {
    type: 'composite';
    mode: CompositeMode;
    allow_multiple: boolean;
    variants: UiVariant[];
  }
  | { type: 'object'; children: UiNode[]; required: string[] };

export interface UiNode {
  pointer: string;
  title?: string | null;
  description?: string | null;
  required: boolean;
  default_value?: JsonValue | null;
  kind: UiNodeKind;
}

export interface UiVariant {
  id: string;
  title?: string | null;
  description?: string | null;
  is_object: boolean;
  node: UiNodeKind;
  schema: JsonValue;
}

export interface UiAst {
  roots: UiNode[];
}
