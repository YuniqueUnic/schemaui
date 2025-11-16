import type { JsonValue, UiAst, UiNode, UiNodeKind, UiVariant } from './types';
import { deepClone, getPointerValue, mergeDefaults, setPointerValue } from './utils/jsonPointer';

export function applyUiDefaults(ast: UiAst | undefined, data: JsonValue): JsonValue {
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
  return defaultForKind(variant.node);
}

export function defaultForKind(kind: UiNodeKind): JsonValue {
  switch (kind.type) {
    case 'field': {
      if (kind.enum_options?.length) {
        return kind.enum_options[0] as JsonValue;
      }
      switch (kind.scalar) {
        case 'integer':
        case 'number':
          return 0;
        case 'boolean':
          return false;
        case 'string':
        default:
          return '';
      }
    }
    case 'array':
      return [];
    case 'object':
      return {};
    case 'composite':
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

function collectDefaults(node: UiNode, store: Record<string, JsonValue | undefined>) {
  if (node.pointer) {
    const fallback = node.default_value ?? defaultForKind(node.kind);
    store[node.pointer] = deepClone(fallback);
  }

  if (node.kind.type === 'array') {
    // default entry defaults to item default
    if (node.kind.item) {
      const placeholder = defaultForKind(node.kind.item);
      const defaultValue = node.default_value ?? [];
      if (Array.isArray(defaultValue) && defaultValue.length === 0) {
        store[node.pointer] = [placeholder] as JsonValue[];
      }
    }
  }

  if (node.kind.type === 'object') {
    node.kind.children.forEach((child) => collectDefaults(child, store));
  }

  if (node.kind.type === 'composite') {
    node.kind.variants.forEach((variant) => walkVariant(variant, store, node.pointer));
  }
}

function walkVariant(variant: UiVariant, store: Record<string, JsonValue | undefined>, basePointer: string) {
  const defaultValue = variantDefault(variant);
  store[basePointer] = deepClone(defaultValue);
  // if object-like, descend children to write nested defaults
  if (variant.node.type === 'object') {
    variant.node.children.forEach((child) => collectDefaults(child, store));
  }
}

function ensureObjectRoot(value: JsonValue): JsonValue {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    return {};
  }
  return value;
}
