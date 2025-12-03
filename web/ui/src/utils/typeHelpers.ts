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
  if (value === null || value === undefined) return "empty";
  if (typeof value === "string") return value || '""';
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) return `[items: ${value.length}]`;
  if (typeof value === "object") {
    const keys = Object.keys(value as Record<string, JsonValue>);
    return keys.length ? `{ ${keys.slice(0, 3).join(", ")} }` : "{}";
  }
  return "value";
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
