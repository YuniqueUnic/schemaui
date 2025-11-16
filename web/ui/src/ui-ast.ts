import type {
  JsonValue,
  WebBlueprint,
  WebCompositeVariant,
  WebField,
  WebFieldKind,
  WebRoot,
  WebSection,
} from "./types";
import { inferKindDefault } from "./utils/defaults";
import { deepClone } from "./utils/jsonPointer";

export interface UiAst {
  roots: UiRootNode[];
}

export interface UiRootNode {
  id: string;
  title?: string | null;
  description?: string | null;
  sections: UiSection[];
}

export interface UiSection {
  id: string;
  title?: string | null;
  description?: string | null;
  children: UiNode[];
  sections: UiSection[];
}

export type UiNode =
  | UiFieldNode
  | UiArrayNode
  | UiCompositeNode
  | UiKeyValueNode;

interface UiNodeBase {
  id: string;
  name: string;
  label: string;
  pointer: string;
  description?: string | null;
  required: boolean;
  defaultValue: JsonValue;
}

export interface UiFieldNode extends UiNodeBase {
  kind: "field";
  fieldType: "string" | "integer" | "number" | "boolean" | "enum" | "json";
  options?: string[];
}

export interface UiArrayNode extends UiNodeBase {
  kind: "array";
  items: WebFieldKind;
  minItems?: number;
  maxItems?: number;
}

type CompositeMode = "one_of" | "any_of" | "all_of";

export interface UiCompositeNode extends UiNodeBase {
  kind: "composite";
  mode: CompositeMode;
  variants: WebCompositeVariant[];
}

type KeyValueKind = Extract<WebFieldKind, { type: "key_value" }>;

export interface UiKeyValueNode extends UiNodeBase {
  kind: "key_value";
  spec: KeyValueKind;
}

export function buildUiAst(blueprint?: WebBlueprint | null): UiAst {
  if (!blueprint) {
    return { roots: [] };
  }
  return {
    roots: blueprint.roots.map((root, index) => mapRoot(root, index)),
  };
}

function mapRoot(root: WebRoot, index: number): UiRootNode {
  return {
    id: root.id || `root-${index}`,
    title: root.title,
    description: root.description,
    sections: (root.sections ?? []).map((section) =>
      mapSection(section)),
  };
}

function mapSection(section: WebSection): UiSection {
  return {
    id: section.id,
    title: section.title,
    description: section.description,
    children: (section.fields ?? []).map((field) => toUiNode(field)),
    sections: (section.sections ?? []).map((child) => mapSection(child)),
  };
}

export function toUiNode(field: WebField): UiNode {
  const base: UiNodeBase = {
    id: field.pointer || field.name,
    name: field.name,
    label: field.label || field.name,
    pointer: field.pointer || "/",
    description: field.description,
    required: field.required,
    defaultValue: resolveDefaultValue(field),
  };

  switch (field.kind.type) {
    case "string":
    case "integer":
    case "number":
    case "boolean":
      return {
        ...base,
        kind: "field",
        fieldType: field.kind.type,
      };
    case "enum":
      return {
        ...base,
        kind: "field",
        fieldType: "enum",
        options: field.kind.options ?? [],
      };
    case "json":
      return {
        ...base,
        kind: "field",
        fieldType: "json",
      };
    case "array":
      return {
        ...base,
        kind: "array",
        items: field.kind.items,
      };
    case "composite":
      return {
        ...base,
        kind: "composite",
        mode: (field.kind.mode ?? "one_of") as CompositeMode,
        variants: field.kind.variants ?? [],
      };
    case "key_value":
      return {
        ...base,
        kind: "key_value",
        spec: field.kind,
      };
    default:
      return {
        ...base,
        kind: "field",
        fieldType: "json",
      };
  }
}

function resolveDefaultValue(field: WebField): JsonValue {
  if (field.default_value !== undefined && field.default_value !== null) {
    return deepClone(field.default_value);
  }
  return deepClone(inferKindDefault(field.kind));
}
