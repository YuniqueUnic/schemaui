import type { FieldError as ServerFieldError } from '@schemaui/types/FieldError';
import type { PreviewRequest as ServerPreviewRequest } from '@schemaui/types/PreviewRequest';
import type { PreviewResponse as ServerPreviewResponse } from '@schemaui/types/PreviewResponse';
import type { SaveRequest as ServerSaveRequest } from '@schemaui/types/SaveRequest';
import type { SessionResponse as ServerSessionResponse } from '@schemaui/types/SessionResponse';
import type { ExitRequest as ServerExitRequest } from '@schemaui/types/ExitRequest';
import type { ValidateRequest as ServerValidateRequest } from '@schemaui/types/ValidateRequest';
import type { ValidationResponse as ServerValidationResponse } from '@schemaui/types/ValidationResponse';
import type { WebBlueprint as GeneratedWebBlueprint } from '@schemaui/types/WebBlueprint';
import type { WebCompositeVariant as GeneratedWebCompositeVariant } from '@schemaui/types/WebCompositeVariant';
import type { WebField as GeneratedWebField } from '@schemaui/types/WebField';
import type { WebFieldKind as GeneratedWebFieldKind } from '@schemaui/types/WebFieldKind';
import type { WebRoot as GeneratedWebRoot } from '@schemaui/types/WebRoot';
import type { WebSection as GeneratedWebSection } from '@schemaui/types/WebSection';

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

export type SessionResponse = Omit<ServerSessionResponse, 'data' | 'blueprint'> & {
  data: JsonValue;
  blueprint: WebBlueprint;
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

type GeneratedCompositeKind = Extract<GeneratedWebFieldKind, { type: 'composite' }>;
type GeneratedArrayKind = Extract<GeneratedWebFieldKind, { type: 'array' }>;
type GeneratedKeyValueKind = Extract<GeneratedWebFieldKind, { type: 'key_value' }>;

export type WebFieldKind =
  | Exclude<
    GeneratedWebFieldKind,
    { type: 'composite' } | { type: 'array' } | { type: 'key_value' }
  >
  | (Omit<GeneratedCompositeKind, 'variants'> & { variants: WebCompositeVariant[] })
  | (Omit<GeneratedArrayKind, 'items'> & { items: WebFieldKind })
  | (Omit<GeneratedKeyValueKind, 'value_kind'> & { value_kind: WebFieldKind });

export interface WebField extends Omit<GeneratedWebField, 'default_value' | 'kind'> {
  default_value?: JsonValue;
  kind: WebFieldKind;
}

export interface WebCompositeVariant
  extends Omit<GeneratedWebCompositeVariant, 'schema' | 'sections'> {
  schema: JsonValue;
  sections: WebSection[];
}

export interface WebSection
  extends Omit<GeneratedWebSection, 'fields' | 'sections'> {
  fields: WebField[];
  sections: WebSection[];
}

export interface WebRoot extends Omit<GeneratedWebRoot, 'sections'> {
  sections: WebSection[];
}

export interface WebBlueprint extends Omit<GeneratedWebBlueprint, 'roots'> {
  roots: WebRoot[];
}
