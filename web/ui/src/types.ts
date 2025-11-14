export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface WebBlueprint {
  title?: string | null;
  description?: string | null;
  roots: WebRoot[];
}

export interface WebRoot {
  id: string;
  title: string;
  description?: string | null;
  sections: WebSection[];
}

export interface WebSection {
  id: string;
  title: string;
  description?: string | null;
  fields: WebField[];
  sections: WebSection[];
}

export interface WebField {
  name: string;
  label: string;
  pointer: string;
  description?: string | null;
  required: boolean;
  default_value?: JsonValue;
  kind: WebFieldKind;
}

export type WebFieldKind =
  | { type: 'string' }
  | { type: 'integer' }
  | { type: 'number' }
  | { type: 'boolean' }
  | { type: 'enum'; options: string[] }
  | { type: 'array'; items?: WebFieldKind }
  | { type: 'json' }
  | {
      type: 'composite';
      mode: 'one_of' | 'any_of';
      variants: WebCompositeVariant[];
    }
  | {
      type: 'key_value';
      key_title: string;
      key_description?: string | null;
      value_title: string;
      value_description?: string | null;
      value_kind: WebFieldKind;
    };

export interface WebCompositeVariant {
  id: string;
  title: string;
  description?: string | null;
  schema: JsonValue;
  is_object: boolean;
}

export interface SessionResponse {
  title?: string | null;
  blueprint: WebBlueprint;
  data: JsonValue;
  formats: string[];
}

export interface FieldError {
  pointer: string;
  message: string;
}

export interface ValidationResponse {
  ok: boolean;
  errors: FieldError[];
}

export interface PreviewResponse {
  payload: string;
}
