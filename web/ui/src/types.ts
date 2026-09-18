import type { FieldError as ServerFieldError } from "@schemaui/types/FieldError";
import type { PreviewRequest as ServerPreviewRequest } from "@schemaui/types/PreviewRequest";
import type { PreviewResponse as ServerPreviewResponse } from "@schemaui/types/PreviewResponse";
import type { SaveRequest as ServerSaveRequest } from "@schemaui/types/SaveRequest";
import type { SessionResponse as ServerSessionResponse } from "@schemaui/types/SessionResponse";
import type { ExitRequest as ServerExitRequest } from "@schemaui/types/ExitRequest";
import type { ValidateRequest as ServerValidateRequest } from "@schemaui/types/ValidateRequest";
import type { ValidationResponse as ServerValidationResponse } from "@schemaui/types/ValidationResponse";

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export type FieldError = ServerFieldError;

export type ValidationResponse = Omit<ServerValidationResponse, "errors"> & {
  errors: FieldError[];
};

export type SessionResponse =
  & Omit<
    ServerSessionResponse,
    "data" | "ui_ast" | "layout" | "blueprint"
  >
  & {
    data: JsonValue;
    ui_ast: UiAst;
    layout?: UiLayout | null;
  };

export type SaveRequest = Omit<ServerSaveRequest, "data"> & {
  data: JsonValue;
};

export type ExitRequest = Omit<ServerExitRequest, "data"> & {
  data: JsonValue;
};

export type ValidateRequest = Omit<ServerValidateRequest, "data"> & {
  data: JsonValue;
};

export type PreviewRequest = Omit<ServerPreviewRequest, "data"> & {
  data: JsonValue;
};

export type PreviewResponse = ServerPreviewResponse;

export type ScalarKind = "string" | "integer" | "number" | "boolean";
export type CompositeMode = "one_of" | "any_of";

export type UiNodeKind =
  | {
    type: "field";
    scalar: ScalarKind;
    enum_options?: string[] | null;
    enum_values?: JsonValue[] | null;
    nullable?: boolean;
    multiline?: boolean;
  }
  | {
    type: "array";
    item: UiNodeKind;
    min_items?: number | null;
    max_items?: number | null;
  }
  | {
    type: "key_value";
    template: UiKeyValueNode;
  }
  | {
    type: "composite";
    mode: CompositeMode;
    allow_multiple: boolean;
    variants: UiVariant[];
  }
  | { type: "object"; children: UiNode[]; required: string[] };

/**
 * Conditional visibility, tested against a sibling property of the object that
 * owns the node. Declared in the schema with `x-visible-when`.
 *
 * `equals` compares the sibling value as a whole; `contains` asks whether an
 * array-valued sibling holds the value as one of its members.
 */
export interface VisibleWhen {
  field: string;
  op: "equals" | "contains";
  value: JsonValue;
}

/**
 * The control an author asked for with `x-control`.
 *
 * Absent means "you choose" — pick the default for the node's shape. A control
 * this build does not know about must also fall back to that default rather
 * than render nothing, so adding a name here is the only way to opt into a
 * bespoke renderer.
 */
export type FieldControl =
  | "text"
  | "textarea"
  | "select"
  | "segmented"
  | "radio"
  | "switch"
  | "checkbox"
  | "slider"
  | "range"
  | "color";

/** A labelled stop along a slider track, from `x-slider-marks`. */
export interface SliderMark {
  value: number;
  label?: string | null;
}

/**
 * Range and scale for a ranged control, read from `minimum`, `maximum`,
 * `multipleOf` and `x-slider-marks`.
 */
export interface FieldBounds {
  minimum?: number | null;
  maximum?: number | null;
  step?: number | null;
  marks?: SliderMark[];
}

export interface UiNode {
  pointer: string;
  title?: string | null;
  description?: string | null;
  required: boolean;
  default_value?: JsonValue | null;
  visible_when?: VisibleWhen | null;
  control?: FieldControl | null;
  bounds?: FieldBounds | null;
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

export interface UiKeyValueNode {
  key_title: string;
  key_description?: string | null;
  key_default?: JsonValue | null;
  key_schema: JsonValue;
  value_title: string;
  value_description?: string | null;
  value_default?: JsonValue | null;
  value_schema: JsonValue;
  value_kind: UiNodeKind;
  entry_schema: JsonValue;
}

export interface UiAst {
  roots: UiNode[];
}

export interface UiLayout {
  roots: LayoutRoot[];
}

export interface LayoutRoot {
  id: string;
  title: string | null;
  description: string | null;
  sections: LayoutSection[];
}

export interface LayoutSection {
  id: string;
  title: string;
  description: string | null;
  pointer: string;
  path: string[];
  field_pointers: string[];
  children: LayoutSection[];
}
