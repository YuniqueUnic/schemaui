import type { JsonValue, UiAst, UiNode, UiNodeKind, UiVariant } from "./types";
import {
  deepClone,
  getPointerValue,
  setPointerValue,
} from "./utils/jsonPointer";
import {
  materializeVariantNode,
  resolveVariantPropertySchemas,
} from "./utils/schemaToUiKind";
import {
  extractSchemaProperties,
  identifyVariantType,
} from "./utils/variantHelpers";
import { VARIANT_CONFIGS } from "./constants/variantDefaults";

export function applyUiDefaults(
  ast: UiAst | undefined,
  data: JsonValue,
): JsonValue {
  let result: JsonValue = deepClone(ensureObjectRoot(data));
  if (!ast) return result;

  for (const node of ast.roots) {
    result = seedRequiredDefaults(node, result);
  }
  return result;
}

export function variantDefault(variant: UiVariant): JsonValue {
  const variantNode = materializeVariantNode(variant);

  // For object variants, use configurable variant identification
  if (variantNode.type === "object") {
    const result: Record<string, JsonValue> = {};
    const resolvedPropertySchemas = resolveVariantPropertySchemas(
      variant.schema,
    );
    const hasSchemaProperties = Object.keys(resolvedPropertySchemas).length > 0;

    // First, set any const fields from the schema (including via $ref / allOf)
    // to ensure unique identification
    for (const [key, propSchema] of Object.entries(resolvedPropertySchemas)) {
      if (propSchema.const !== undefined) {
        result[key] = deepClone(propSchema.const);
      }
    }

    // Identify variant type using configuration
    const schemaProperties = hasSchemaProperties
      ? resolvedPropertySchemas
      : extractSchemaProperties(variant.schema);
    const variantType = identifyVariantType(schemaProperties);
    const variantConfig = variantType ? VARIANT_CONFIGS[variantType] : null;
    const requiredSet = new Set(variantNode.required ?? []);

    // Process each child node: include only schema-authored defaults or required fields.
    for (const child of variantNode.children) {
      const key = child.pointer.split("/").pop() || "";
      if (!key || key in result) continue;

      const propSchema = resolvedPropertySchemas[key];
      const authoredDefault = propSchema !== undefined
        ? (propSchema.default !== undefined
          ? propSchema.default
          : propSchema.const !== undefined
          ? propSchema.const
          : undefined)
        : (!hasSchemaProperties && child.default_value !== null
          ? child.default_value
          : undefined);

      const isRequired = child.required || requiredSet.has(key);
      if (authoredDefault !== undefined) {
        result[key] = deepClone(authoredDefault);
      } else if (isRequired) {
        if (variantConfig && key in variantConfig.defaults) {
          result[key] = deepClone(variantConfig.defaults[key] as JsonValue);
        } else {
          result[key] = deepClone(
            child.default_value ?? defaultForKind(child.kind),
          );
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
            result[requiredKey] = deepClone(
              childNode.default_value ?? defaultForKind(childNode.kind),
            );
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
      if (kind.nullable) {
        return null;
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
    case "key_value":
      return {};
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

function seedRequiredDefaults(node: UiNode, root: JsonValue): JsonValue {
  let result = root;

  if (node.kind.type === "object") {
    const current = node.pointer ? getPointerValue(result, node.pointer) : result;
    if (current === null) {
      return result;
    }

    if (current === undefined) {
      if (!node.required) {
        return result;
      }
      const fallback = node.default_value ?? defaultForKind(node.kind);
      result = setPointerValue(result, node.pointer, deepClone(fallback));
    }

    const nextCurrent = node.pointer
      ? getPointerValue(result, node.pointer)
      : result;
    if (
      nextCurrent !== null &&
      typeof nextCurrent === "object" &&
      !Array.isArray(nextCurrent)
    ) {
      for (const child of node.kind.children) {
        result = seedRequiredDefaults(child, result);
      }
    }
    return result;
  }

  if (node.pointer && node.required) {
    const current = getPointerValue(result, node.pointer);
    if (current === undefined) {
      const fallback = node.default_value ?? defaultForKind(node.kind);
      result = setPointerValue(result, node.pointer, deepClone(fallback));
    }
  }

  return result;
}

function ensureObjectRoot(value: JsonValue): JsonValue {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  return value;
}
