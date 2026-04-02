import type { JsonValue } from "../types";
import { deepEqual } from "./deepEqual";

type JsonSchema = JsonValue;

interface MatchResult {
  matched: boolean;
  score: number;
}

export function variantMatches(
  value: JsonValue | undefined,
  schema: JsonSchema | undefined,
): boolean {
  return variantMatchScore(value, schema) !== null;
}

export function variantMatchScore(
  value: JsonValue | undefined,
  schema: JsonSchema | undefined,
): number | null {
  if (!schema || typeof schema !== "object" || Array.isArray(schema)) {
    return null;
  }

  const result = matchSchema(value, schema as Record<string, JsonValue>);
  return result.matched ? result.score : null;
}

function matchSchema(
  value: JsonValue | undefined,
  schema: Record<string, JsonValue>,
): MatchResult {
  if (schema.const !== undefined) {
    return deepEqual(value, schema.const as JsonValue)
      ? { matched: true, score: 200 }
      : noMatch();
  }
  if (Array.isArray(schema.enum)) {
    return (schema.enum as JsonValue[]).some((entry) =>
        deepEqual(entry, value)
      )
      ? { matched: true, score: 150 }
      : noMatch();
  }

  if (Array.isArray(schema.allOf)) {
    let score = 40;
    for (const sub of schema.allOf as JsonSchema[]) {
      const result = matchSchema(value, ensureObject(sub));
      if (!result.matched) {
        return noMatch();
      }
      score += result.score;
    }
    return { matched: true, score };
  }
  if (Array.isArray(schema.oneOf)) {
    const matches = (schema.oneOf as JsonSchema[])
      .map((sub) => matchSchema(value, ensureObject(sub)))
      .filter((result) => result.matched);
    return matches.length === 1
      ? { matched: true, score: 60 + matches[0].score }
      : noMatch();
  }
  if (Array.isArray(schema.anyOf)) {
    const matches = (schema.anyOf as JsonSchema[])
      .map((sub) => matchSchema(value, ensureObject(sub)))
      .filter((result) => result.matched);
    if (matches.length === 0) {
      return noMatch();
    }
    return {
      matched: true,
      score: 50 + Math.max(...matches.map((result) => result.score)),
    };
  }

  const normalizedType = normalizeType(schema.type);
  if (normalizedType === "object" || schema.properties || schema.required) {
    return matchObject(value, schema);
  }
  if (normalizedType === "array" || schema.items) {
    return matchArray(value, schema);
  }

  if (normalizedType) {
    return typeMatches(value, normalizedType)
      ? { matched: true, score: 100 }
      : noMatch();
  }

  return { matched: true, score: 10 };
}

function matchObject(
  value: JsonValue | undefined,
  schema: Record<string, JsonValue>,
): MatchResult {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return noMatch();
  }
  const obj = value as Record<string, JsonValue>;
  let score = 120;

  // Step 1: Check const fields - these are discriminators for variant matching
  // If any const field doesn't match, this is definitely not the right variant
  if (schema.properties && typeof schema.properties === "object") {
    const props = schema.properties as Record<string, JsonValue>;
    for (const [key, propSchema] of Object.entries(props)) {
      if (!propSchema || typeof propSchema !== "object") {
        continue;
      }
      if ("const" in propSchema) {
        if (!deepEqual(obj[key], propSchema.const)) {
          return noMatch(); // Const field mismatch - definitely wrong variant
        }
        score += 80;
      }
    }
  }

  // Step 2: Check required fields
  const required = Array.isArray(schema.required)
    ? (schema.required as JsonValue[])
    : [];
  for (const key of required) {
    if (typeof key === "string" && !(key in obj)) {
      return noMatch(); // Missing required field
    }
    score += 25;
  }

  // Step 3: Check object field compatibility against schema properties.
  if (schema.properties && typeof schema.properties === "object") {
    const props = schema.properties as Record<string, JsonValue>;
    const schemaKeys = Object.keys(props);
    const objectKeys = Object.keys(obj);

    // Check for field compatibility between object and schema
    // Be strict about unexpected fields for better variant discrimination
    let unexpectedFieldCount = 0;

    for (const objKey of objectKeys) {
      if (!schemaKeys.includes(objKey)) {
        unexpectedFieldCount++;
      }
    }

    // If object has fields not in this schema, it's likely the wrong variant
    // This is critical for discriminating between variants in oneOf/anyOf
    if (unexpectedFieldCount > 0) {
      if (schema.additionalProperties !== true) {
        return noMatch();
      }
      score -= unexpectedFieldCount * 5;
    }

    // Check property schemas
    for (const [key, propSchema] of Object.entries(props)) {
      if (!propSchema || typeof propSchema !== "object") {
        continue;
      }
      if ("const" in propSchema) {
        continue; // Already checked const fields
      }

      // If the field exists in the object, it must match the schema
      if (obj[key] !== undefined) {
        const result = matchSchema(obj[key], propSchema as Record<string, JsonValue>);
        if (!result.matched) {
          return noMatch();
        }
        score += 20 + result.score;
      } else if (required.includes(key)) {
        return noMatch(); // Required field is missing
      }
    }
  }

  return { matched: true, score };
}

function matchArray(
  value: JsonValue | undefined,
  schema: Record<string, JsonValue>,
): MatchResult {
  if (!Array.isArray(value)) {
    return noMatch();
  }
  let score = 110 + value.length;
  if (!schema.items) {
    return { matched: true, score };
  }
  if (Array.isArray(schema.items)) {
    if (schema.items.length === 0) {
      return { matched: true, score };
    }
    const result = matchSchema(value[0], ensureObject(schema.items[0]));
    return result.matched
      ? { matched: true, score: score + result.score }
      : noMatch();
  }
  for (const entry of value) {
    const result = matchSchema(entry, ensureObject(schema.items));
    if (!result.matched) {
      return noMatch();
    }
    score += result.score;
  }
  return { matched: true, score };
}

function ensureObject(value: JsonValue | undefined): Record<string, JsonValue> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  return value as Record<string, JsonValue>;
}

function normalizeType(typeValue: JsonValue | undefined): string | undefined {
  if (typeof typeValue === "string") {
    return typeValue;
  }
  if (Array.isArray(typeValue)) {
    return typeValue.find((entry) => typeof entry === "string") as
      | string
      | undefined;
  }
  return undefined;
}

function typeMatches(value: JsonValue | undefined, expected: string): boolean {
  switch (expected) {
    case "string":
      return typeof value === "string";
    case "integer":
      return typeof value === "number" && Number.isInteger(value);
    case "number":
      return typeof value === "number";
    case "boolean":
      return typeof value === "boolean";
    case "array":
      return Array.isArray(value);
    case "object":
      return typeof value === "object" && value !== null &&
        !Array.isArray(value);
    default:
      return true;
  }
}

function noMatch(): MatchResult {
  return { matched: false, score: Number.NEGATIVE_INFINITY };
}
