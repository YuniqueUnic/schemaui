import type {
  JsonValue,
  WebBlueprint,
  WebField,
  WebFieldKind,
  WebSection,
} from "../types";
import {
  deepClone,
  getPointerValue,
  mergeDefaults,
  setPointerValue,
} from "./jsonPointer";

export function applyBlueprintDefaults(
  blueprint: WebBlueprint | undefined,
  data: JsonValue,
): JsonValue {
  if (!blueprint) {
    return ensureObjectRoot(data);
  }

  const defaults: Record<string, JsonValue | undefined> = {};
  const fallbackByPointer = new Map<string, JsonValue>();

  blueprint.roots.forEach((root) => {
    root.sections?.forEach((section) =>
      collectSectionDefaults(section, defaults, fallbackByPointer)
    );
  });

  let result = mergeDefaults(ensureObjectRoot(data), defaults);
  fallbackByPointer.forEach((fallback, pointer) => {
    const current = getPointerValue(result, pointer);
    if (current === null || current === undefined) {
      result = setPointerValue(result, pointer, deepClone(fallback));
    }
  });
  return result;
}

function collectSectionDefaults(
  section: WebSection,
  defaults: Record<string, JsonValue | undefined>,
  fallbackByPointer: Map<string, JsonValue>,
) {
  section.fields?.forEach((field) =>
    recordDefault(field, defaults, fallbackByPointer)
  );
  section.sections?.forEach((child) =>
    collectSectionDefaults(child, defaults, fallbackByPointer)
  );
}

function recordDefault(
  field: WebField,
  defaults: Record<string, JsonValue | undefined>,
  fallbackByPointer: Map<string, JsonValue>,
) {
  if (!field.pointer) {
    return;
  }
  const fallback = resolveDefaultValue(field);
  defaults[field.pointer] = deepClone(fallback);
  fallbackByPointer.set(field.pointer, fallback);
}

function resolveDefaultValue(field: WebField): JsonValue {
  return field.default_value ?? inferKindDefault(field.kind);
}

function inferKindDefault(kind: WebFieldKind): JsonValue {
  switch (kind.type) {
    case "string":
      return "";
    case "integer":
    case "number":
      return 0;
    case "boolean":
      return false;
    case "enum":
      return kind.options[0] ?? "";
    case "array":
      return [];
    case "json":
      return {};
    case "key_value":
      return {};
    case "composite":
      return kind.mode === "any_of" ? [] : {};
    default:
      return {};
  }
}

function ensureObjectRoot(value: JsonValue): JsonValue {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  return value;
}
