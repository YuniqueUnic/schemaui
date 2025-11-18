import type { JsonValue, UiAst, UiNode, UiNodeKind, UiVariant } from "./types";
import {
  deepClone,
  getPointerValue,
  mergeDefaults,
  setPointerValue,
} from "./utils/jsonPointer";

export function applyUiDefaults(
  ast: UiAst | undefined,
  data: JsonValue,
): JsonValue {
  if (!ast) return ensureObjectRoot(data);

  const defaults: Record<string, JsonValue | undefined> = {};
  ast.roots.forEach((node) => collectDefaults(node, defaults));

  let result = mergeDefaults(ensureObjectRoot(data), defaults);
  // fallback: if value becomes undefined after merge, write default explicitly
  for (const [pointer, value] of Object.entries(defaults)) {
    if (value === undefined) continue;
    const current = getPointerValue(result, pointer);
    if (current === undefined || current === null) {
      result = setPointerValue(result, pointer, deepClone(value));
    }
  }
  return result;
}

export function variantDefault(variant: UiVariant): JsonValue {
  // For object variants, check if there are const fields that can uniquely identify the variant
  if (variant.node.type === "object") {
    const result: Record<string, JsonValue> = {};

    // First, set any const fields from the schema to ensure unique identification
    if (
      variant.schema && typeof variant.schema === "object" &&
      "properties" in variant.schema
    ) {
      const properties = variant.schema.properties as Record<string, JsonValue>;
      for (const [key, prop] of Object.entries(properties)) {
        if (
          prop && typeof prop === "object" && prop !== null && "const" in prop
        ) {
          result[key] = (prop as { const: JsonValue }).const;
        }
      }
    }

    // Then add defaults for required fields, using unique values when possible
    // Identify variant type by examining schema properties
    const schemaProperties =
      (variant.schema && typeof variant.schema === "object" &&
          !Array.isArray(variant.schema) && "properties" in variant.schema)
        ? (variant.schema.properties as Record<string, unknown>)
        : {};

    // Identify variant types by their property signatures
    const hasLabel = "label" in schemaProperties;
    const hasEnabled = "enabled" in schemaProperties;
    const hasValues = "values" in schemaProperties;
    const hasUrl = "url" in schemaProperties;
    const hasPriority = "priority" in schemaProperties;
    const hasActive = "active" in schemaProperties;

    // simpleItem: {id, label?, enabled?}
    const isSimpleItem = (hasLabel || hasEnabled) && !hasValues && !hasUrl;
    // numericItem: {id, values?}
    const isNumericItem = hasValues && !hasLabel && !hasEnabled;
    // target: {url, priority?, active?}
    const isTarget = hasUrl && (hasPriority || hasActive);

    for (const child of variant.node.children) {
      const key = child.pointer.split("/").pop() || "";
      if (!(key in result)) {
        // Special handling for specific variant types to ensure uniqueness
        if (key === "id") {
          // Generate unique IDs based on the schema reference or variant ID
          if (isSimpleItem) {
            result[key] = 1001; // Unique ID for simpleItem
          } else if (isNumericItem) {
            result[key] = 2001; // Unique ID for numericItem
          } else if (variant.id) {
            // Generic hash-based ID for other variants
            const hash = variant.id.split("").reduce(
              (acc, char) => acc + char.charCodeAt(0),
              0,
            );
            result[key] = hash % 1000;
          } else {
            result[key] = 0;
          }
        } else if (key === "label" && isSimpleItem) {
          // Give simpleItem a default label to distinguish it
          result[key] = "item";
        } else if (key === "values" && isNumericItem) {
          // Give numericItem some default values to distinguish it
          result[key] = [1];
        } else if (key === "url" && isTarget) {
          // For target objects with uri format, provide a valid URL
          result[key] = "https://example.com";
        } else if (key === "priority" && isTarget) {
          // For target objects, set priority within valid range (1-10)
          result[key] = 5;
        } else {
          result[key] = child.default_value ?? defaultForKind(child.kind);
        }
      }
    }

    // Ensure all required fields have values
    if (variant.node.required) {
      for (const requiredKey of variant.node.required) {
        if (!(requiredKey in result)) {
          // Find the child node for this key
          const childNode = variant.node.children.find(
            (c) => c.pointer.split("/").pop() === requiredKey,
          );
          if (childNode) {
            result[requiredKey] = childNode.default_value ??
              defaultForKind(childNode.kind);
          }
        }
      }
    }

    return result;
  }

  // For array variants, add a sample element to distinguish between string[] and number[]
  if (variant.node.type === "array") {
    const itemDefault = defaultForKind(variant.node.item);
    // Return array with one default element to make it distinguishable
    return [itemDefault];
  }

  return defaultForKind(variant.node);
}

export function defaultForKind(kind: UiNodeKind): JsonValue {
  switch (kind.type) {
    case "field": {
      if (kind.enum_options?.length) {
        return kind.enum_options[0] as JsonValue;
      }
      switch (kind.scalar) {
        case "integer":
        case "number":
          return 0;
        case "boolean":
          return false;
        case "string":
        default:
          return "";
      }
    }
    case "array":
      return [];
    case "object":
      return {};
    case "composite":
      if (kind.allow_multiple) {
        return [];
      }
      if (kind.variants[0]) {
        return variantDefault(kind.variants[0]);
      }
      return {};
    default:
      return {};
  }
}

function collectDefaults(
  node: UiNode,
  store: Record<string, JsonValue | undefined>,
) {
  if (node.pointer) {
    const fallback = node.default_value ?? defaultForKind(node.kind);
    store[node.pointer] = deepClone(fallback);
  }

  if (node.kind.type === "array") {
    // default entry defaults to item default
    if (node.kind.item) {
      const placeholder = defaultForKind(node.kind.item);
      const defaultValue = node.default_value ?? [];
      if (Array.isArray(defaultValue) && defaultValue.length === 0) {
        store[node.pointer] = [placeholder] as JsonValue[];
      }
    }
  }

  if (node.kind.type === "object") {
    node.kind.children.forEach((child) => collectDefaults(child, store));
  }

  if (node.kind.type === "composite") {
    node.kind.variants.forEach((variant) =>
      walkVariant(variant, store, node.pointer)
    );
  }
}

function walkVariant(
  variant: UiVariant,
  store: Record<string, JsonValue | undefined>,
  basePointer: string,
) {
  const defaultValue = variantDefault(variant);
  store[basePointer] = deepClone(defaultValue);
  // Note: We don't recursively collect defaults for variant children here
  // because they should only be applied when this specific variant is active.
  // The variantDefault function already handles generating the complete default
  // object for the variant, including all its required fields.
}

function ensureObjectRoot(value: JsonValue): JsonValue {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  return value;
}
