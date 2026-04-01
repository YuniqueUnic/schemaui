import type { JsonValue, UiAst, UiNode, UiNodeKind, UiVariant } from "./types";
import {
  deepClone,
  getPointerValue,
  mergeDefaults,
  setPointerValue,
} from "./utils/jsonPointer";
import { materializeVariantNode } from "./utils/schemaToUiKind";
import {
  extractSchemaProperties,
  identifyVariantType,
} from "./utils/variantHelpers";
import { VARIANT_CONFIGS } from "./constants/variantDefaults";

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
  const variantNode = materializeVariantNode(variant);

  // For object variants, use configurable variant identification
  if (variantNode.type === "object") {
    const result: Record<string, JsonValue> = {};

    // First, set any const fields from the schema to ensure unique identification
    const schemaProperties = extractSchemaProperties(variant.schema);

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

    // Identify variant type using configuration
    const variantType = identifyVariantType(schemaProperties);
    const variantConfig = variantType ? VARIANT_CONFIGS[variantType] : null;

    // Process each child node
    for (const child of variantNode.children) {
      const key = child.pointer.split("/").pop() || "";
      if (!(key in result)) {
        // Use variant-specific defaults if available
        if (variantConfig && key in variantConfig.defaults) {
          result[key] = variantConfig.defaults[key] as JsonValue;
        } else if (key === "id" && !variantConfig) {
          // Fallback: Generate hash-based ID for unknown variants
          const hash = variant.id.split("").reduce(
            (acc, char) => acc + char.charCodeAt(0),
            0,
          );
          result[key] = hash % 1000;
        } else {
          result[key] = child.default_value ?? defaultForKind(child.kind);
        }
      }
    }

    // Ensure all required fields have values
    if (variantNode.required) {
      for (const requiredKey of variantNode.required) {
        if (!(requiredKey in result)) {
          const childNode = variantNode.children.find(
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
  if (variantNode.type === "array") {
    const itemDefault = defaultForKind(variantNode.item);
    return [itemDefault];
  }

  return defaultForKind(variantNode);
}

export function defaultForKind(kind: UiNodeKind): JsonValue {
  switch (kind.type) {
    case "field": {
      if (kind.enum_values?.length) {
        return deepClone(kind.enum_values[0] as JsonValue);
      }
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
    // Use backend-provided default_value if available, otherwise use defaultForKind
    // For arrays: respect backend semantics - empty arrays stay empty unless minItems > 0
    const fallback = node.default_value ?? defaultForKind(node.kind);
    store[node.pointer] = deepClone(fallback);
  }

  // NOTE: Removed the array-specific block that forced [placeholder] for empty arrays
  // This was causing [] to become [""] which blurs the semantic difference between
  // "no items" and "one empty item". User must explicitly click "Add entry" to add items.

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
