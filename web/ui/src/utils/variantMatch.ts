import type { JsonValue } from "../types";

type JsonSchema = JsonValue;

export function variantMatches(
  value: JsonValue | undefined,
  schema: JsonSchema | undefined,
): boolean {
  if (!schema || typeof schema !== "object" || Array.isArray(schema)) {
    return false;
  }

  return matchSchema(value, schema as Record<string, JsonValue>);
}

function matchSchema(
  value: JsonValue | undefined,
  schema: Record<string, JsonValue>,
): boolean {
  if (schema.const !== undefined) {
    return deepEqual(value, schema.const as JsonValue);
  }
  if (Array.isArray(schema.enum)) {
    return (schema.enum as JsonValue[]).some((entry) =>
      deepEqual(entry, value)
    );
  }

  if (Array.isArray(schema.allOf)) {
    return (schema.allOf as JsonSchema[]).every((sub) =>
      matchSchema(value, ensureObject(sub))
    );
  }
  if (Array.isArray(schema.oneOf)) {
    const matches = (schema.oneOf as JsonSchema[]).filter((sub) =>
      matchSchema(value, ensureObject(sub))
    );
    return matches.length === 1;
  }
  if (Array.isArray(schema.anyOf)) {
    return (schema.anyOf as JsonSchema[]).some((sub) =>
      matchSchema(value, ensureObject(sub))
    );
  }

  const normalizedType = normalizeType(schema.type);
  if (normalizedType === "object" || schema.properties || schema.required) {
    return matchObject(value, schema);
  }
  if (normalizedType === "array" || schema.items) {
    return matchArray(value, schema);
  }

  if (normalizedType) {
    return typeMatches(value, normalizedType);
  }

  return true;
}

function matchObject(
  value: JsonValue | undefined,
  schema: Record<string, JsonValue>,
): boolean {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  const obj = value as Record<string, JsonValue>;

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
          return false; // Const field mismatch - definitely wrong variant
        }
      }
    }
  }

  // Step 2: Check required fields
  const required = Array.isArray(schema.required)
    ? (schema.required as JsonValue[])
    : [];
  for (const key of required) {
    if (typeof key === "string" && !(key in obj)) {
      return false; // Missing required field
    }
  }

  // Step 3: For stricter matching, check if the object has only the expected fields
  // This helps distinguish between variants that have different field sets
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
      // Always be strict about unexpected fields for variant matching
      // This helps distinguish simpleItem (id, label, enabled) from numericItem (id, values)
      // The only exception is if additionalProperties is explicitly true
      if (schema.additionalProperties !== true) {
        return false;
      }
    }

    // Also check if this schema expects fields that the object doesn't have
    // This helps distinguish variants with different field sets
    const missingExpectedFields = schemaKeys.filter((key) => !(key in obj));

    // Check if any missing fields are required - if so, immediate fail
    for (const missingKey of missingExpectedFields) {
      if (required.includes(missingKey)) {
        return false; // Missing required field
      }
    }

    // For variant matching: if object has ALL the properties it needs
    // but schema expects additional properties the object doesn't have,
    // this might not be the right variant
    // This helps distinguish simpleItem (id, label, enabled) from numericItem (id, values)
    if (missingExpectedFields.length > 0 && unexpectedFieldCount === 0) {
      // Object perfectly matches a subset of this schema's fields
      // But is missing some optional fields - this is suspicious for variant matching
      // Only allow this if we have explicit evidence this is the right variant

      // If object has fields NOT in this schema, definitely wrong variant
      // But we already checked that above (unexpectedFieldCount === 0 here)

      // If we have many missing fields, probably wrong variant
      if (missingExpectedFields.length > schemaKeys.length * 0.5) {
        return false; // Too many missing fields
      }
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
        if (!matchSchema(obj[key], propSchema as Record<string, JsonValue>)) {
          return false;
        }
      } else if (required.includes(key)) {
        return false; // Required field is missing
      }
    }
  }

  return true;
}

function matchArray(
  value: JsonValue | undefined,
  schema: Record<string, JsonValue>,
): boolean {
  if (!Array.isArray(value)) {
    return false;
  }
  if (!schema.items) {
    return true;
  }
  if (Array.isArray(schema.items)) {
    return schema.items.length === 0 ||
      matchSchema(value[0], ensureObject(schema.items[0]));
  }
  return value.every((entry) => matchSchema(entry, ensureObject(schema.items)));
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

function deepEqual(
  a: JsonValue | undefined,
  b: JsonValue | undefined,
): boolean {
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
  if (a && typeof a === "object" && b && typeof b === "object") {
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    if (aKeys.length !== bKeys.length) return false;
    return aKeys.every((key) =>
      deepEqual(
        (a as Record<string, JsonValue>)[key],
        (b as Record<string, JsonValue>)[key],
      )
    );
  }
  return false;
}
