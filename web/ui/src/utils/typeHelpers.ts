import type { JsonValue, UiNodeKind } from "../types";

/**
 * Type classification and inference utilities
 */

/**
 * Determines if a UI node kind represents a simple, primitive type
 * that can be rendered inline without requiring a dialog/overlay.
 */
export function isSimpleKind(kind: UiNodeKind): boolean {
  return kind.type === "field" && !kind.enum_options;
}

/**
 * Infers the type of a value for display purposes
 */
export function inferValueType(value: JsonValue | undefined): string {
  if (value === null || value === undefined) return "null";
  if (typeof value === "string") return "string";
  if (typeof value === "number") {
    return Number.isInteger(value) ? "integer" : "number";
  }
  if (typeof value === "boolean") return "boolean";
  if (Array.isArray(value)) return "array";
  if (typeof value === "object") return "object";
  return "unknown";
}

/**
 * Formats a value for summary display
 */
export function formatValueSummary(value: JsonValue | undefined): string {
  return formatSummary(value, 0);
}

/**
 * Extracts a child value from a container using a pointer segment
 */
export function extractChildValue(
  container: JsonValue | undefined,
  pointer: string,
): JsonValue | undefined {
  if (container === null || container === undefined) {
    return undefined;
  }
  const token = pointerSegment(pointer);
  if (!token) {
    return undefined;
  }
  if (Array.isArray(container)) {
    const index = Number(token);
    return Number.isNaN(index) ? undefined : container[index];
  }
  if (typeof container === "object") {
    return (container as Record<string, JsonValue>)[token];
  }
  return undefined;
}

/**
 * Extracts the last segment from a JSON pointer
 */
function pointerSegment(pointer: string): string | undefined {
  if (!pointer || pointer === "/") {
    return undefined;
  }
  const segments = pointer.split("/").filter(Boolean);
  const raw = segments[segments.length - 1];
  return raw?.replace(/~1/g, "/").replace(/~0/g, "~");
}

const MAX_OBJECT_ENTRIES = 3;
const MAX_ARRAY_ITEMS = 3;
const MAX_DEPTH = 2;
const MAX_STRING_LENGTH = 28;

function formatSummary(
  value: JsonValue | undefined,
  depth: number,
): string {
  if (value === undefined) return "empty";
  if (value === null) return "null";
  if (typeof value === "string") {
    return value.length === 0
      ? '""'
      : value.length > MAX_STRING_LENGTH
      ? `${value.slice(0, MAX_STRING_LENGTH - 1)}…`
      : value;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) {
    if (value.length === 0) return "[]";
    if (depth >= MAX_DEPTH) {
      return `[${value.length} items]`;
    }

    const items = value.slice(0, MAX_ARRAY_ITEMS).map((entry) =>
      formatSummary(entry, depth + 1)
    );
    const suffix = value.length > MAX_ARRAY_ITEMS
      ? `, +${value.length - MAX_ARRAY_ITEMS} more`
      : "";
    return `[${items.join(", ")}${suffix}]`;
  }
  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, JsonValue>);
    if (entries.length === 0) return "{}";
    if (depth >= MAX_DEPTH) {
      return `{${entries.length} fields}`;
    }

    const summary = entries.slice(0, MAX_OBJECT_ENTRIES).map(([key, entryValue]) =>
      `${key}: ${formatSummary(entryValue, depth + 1)}`
    );
    const suffix = entries.length > MAX_OBJECT_ENTRIES
      ? `, +${entries.length - MAX_OBJECT_ENTRIES} more`
      : "";
    return `{ ${summary.join(", ")}${suffix} }`;
  }
  return "value";
}
