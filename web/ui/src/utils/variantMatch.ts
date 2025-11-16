import type { JsonValue } from '../types';

type JsonSchema = JsonValue;

export function variantMatches(value: JsonValue | undefined, schema: JsonSchema | undefined): boolean {
  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) {
    return false;
  }
  return matchSchema(value, schema as Record<string, JsonValue>);
}

function matchSchema(value: JsonValue | undefined, schema: Record<string, JsonValue>): boolean {
  if (schema.const !== undefined) {
    return deepEqual(value, schema.const as JsonValue);
  }
  if (Array.isArray(schema.enum)) {
    return (schema.enum as JsonValue[]).some((entry) => deepEqual(entry, value));
  }

  if (Array.isArray(schema.allOf)) {
    return (schema.allOf as JsonSchema[]).every((sub) =>
      matchSchema(value, ensureObject(sub)),
    );
  }
  if (Array.isArray(schema.oneOf)) {
    const matches = (schema.oneOf as JsonSchema[]).filter((sub) =>
      matchSchema(value, ensureObject(sub)),
    );
    return matches.length === 1;
  }
  if (Array.isArray(schema.anyOf)) {
    return (schema.anyOf as JsonSchema[]).some((sub) => matchSchema(value, ensureObject(sub)));
  }

  const normalizedType = normalizeType(schema.type);
  if (normalizedType === 'object' || schema.properties || schema.required) {
    return matchObject(value, schema);
  }
  if (normalizedType === 'array' || schema.items) {
    return matchArray(value, schema);
  }

  if (normalizedType) {
    return typeMatches(value, normalizedType);
  }

  return true;
}

function matchObject(value: JsonValue | undefined, schema: Record<string, JsonValue>): boolean {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, JsonValue>;
  const required = Array.isArray(schema.required) ? (schema.required as JsonValue[]) : [];
  for (const key of required) {
    if (typeof key === 'string' && !(key in obj)) {
      return false;
    }
  }
  if (schema.properties && typeof schema.properties === 'object') {
    const props = schema.properties as Record<string, JsonValue>;
    for (const [key, propSchema] of Object.entries(props)) {
      if (!propSchema || typeof propSchema !== 'object') {
        continue;
      }
      if (obj[key] === undefined) {
        continue;
      }
      if (!matchSchema(obj[key], propSchema as Record<string, JsonValue>)) {
        return false;
      }
    }
  }
  return true;
}

function matchArray(value: JsonValue | undefined, schema: Record<string, JsonValue>): boolean {
  if (!Array.isArray(value)) {
    return false;
  }
  if (!schema.items) {
    return true;
  }
  if (Array.isArray(schema.items)) {
    return schema.items.length === 0 || matchSchema(value[0], ensureObject(schema.items[0]));
  }
  return value.every((entry) => matchSchema(entry, ensureObject(schema.items)));
}

function ensureObject(value: JsonValue | undefined): Record<string, JsonValue> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {};
  }
  return value as Record<string, JsonValue>;
}

function normalizeType(typeValue: JsonValue | undefined): string | undefined {
  if (typeof typeValue === 'string') {
    return typeValue;
  }
  if (Array.isArray(typeValue)) {
    return typeValue.find((entry) => typeof entry === 'string') as string | undefined;
  }
  return undefined;
}

function typeMatches(value: JsonValue | undefined, expected: string): boolean {
  switch (expected) {
    case 'string':
      return typeof value === 'string';
    case 'integer':
      return typeof value === 'number' && Number.isInteger(value);
    case 'number':
      return typeof value === 'number';
    case 'boolean':
      return typeof value === 'boolean';
    case 'array':
      return Array.isArray(value);
    case 'object':
      return typeof value === 'object' && value !== null && !Array.isArray(value);
    default:
      return true;
  }
}

function deepEqual(a: JsonValue | undefined, b: JsonValue | undefined): boolean {
  if (a === b) {
    return true;
  }
  if (typeof a !== typeof b) {
    return false;
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    return a.every((entry, index) => deepEqual(entry, b[index]));
  }
  if (a && typeof a === 'object' && b && typeof b === 'object') {
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    if (aKeys.length !== bKeys.length) return false;
    return aKeys.every((key) => deepEqual((a as Record<string, JsonValue>)[key], (b as Record<string, JsonValue>)[key]));
  }
  return false;
}
