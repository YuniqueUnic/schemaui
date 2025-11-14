import type { JsonValue } from '../types';

function decode(segment: string): string {
  return segment.replace(/~1/g, '/').replace(/~0/g, '~');
}

function encode(segment: string): string {
  return segment.replace(/~/g, '~0').replace(/\//g, '~1');
}

export function splitPointer(pointer?: string | null): string[] {
  if (!pointer || pointer === '/' || pointer.length === 0) {
    return [];
  }
  return pointer
    .split('/')
    .slice(1)
    .map((part) => decode(part));
}

export function joinPointer(segments: string[]): string {
  if (segments.length === 0) {
    return '/';
  }
  return `/${segments.map((segment) => encode(segment)).join('/')}`;
}

export function getPointerValue(
  root: JsonValue,
  pointer?: string | null,
): JsonValue | undefined {
  if (!pointer || pointer === '/') {
    return root;
  }

  let current: JsonValue | undefined = root;
  for (const segment of splitPointer(pointer)) {
    if (current === null || current === undefined) {
      return undefined;
    }
    if (Array.isArray(current)) {
      const index = Number(segment);
      current = current[index];
    } else if (typeof current === 'object') {
      current = (current as Record<string, JsonValue>)[segment];
    } else {
      return undefined;
    }
  }
  return current;
}

export function setPointerValue(
  root: JsonValue,
  pointer: string | undefined,
  value: JsonValue,
): JsonValue {
  if (!pointer || pointer === '/') {
    return value;
  }

  const segments = splitPointer(pointer);
  const clone = deepClone(root ?? {});
  let current: JsonValue = clone;

  segments.forEach((segment, index) => {
    const isLast = index === segments.length - 1;
    if (Array.isArray(current)) {
      const idx = Number(segment);
      if (Number.isNaN(idx)) {
        throw new Error(`Invalid array index in pointer: ${segment}`);
      }
      if (isLast) {
        current[idx] = value;
      } else {
        current[idx] = ensureContainer(current[idx], segments[index + 1]);
        current = current[idx];
      }
      return;
    }
    if (typeof current === 'object' && current !== null) {
      const obj = current as Record<string, JsonValue>;
      if (isLast) {
        obj[segment] = value;
      } else {
        obj[segment] = ensureContainer(obj[segment], segments[index + 1]);
        current = obj[segment];
      }
      return;
    }
    throw new Error(`Cannot set pointer on non-container value`);
  });

  return clone;
}

function ensureContainer(
  existing: JsonValue | undefined,
  next: string,
): JsonValue {
  if (existing === undefined || existing === null) {
    return isFinite(Number(next)) ? [] : {};
  }
  if (Array.isArray(existing) || typeof existing === 'object') {
    return deepClone(existing);
  }
  return isFinite(Number(next)) ? [] : {};
}

export function mergeDefaults(
  base: JsonValue,
  defaults: Record<string, JsonValue | undefined>,
): JsonValue {
  let result = deepClone(base ?? {});
  for (const [pointer, value] of Object.entries(defaults)) {
    if (value === undefined) {
      continue;
    }
    const current = getPointerValue(result, pointer);
    if (current === undefined) {
      result = setPointerValue(result, pointer, deepClone(value));
    }
  }
  return result;
}

export function deepClone<T extends JsonValue>(value: T): T {
  return typeof structuredClone === 'function'
    ? structuredClone(value)
    : (JSON.parse(JSON.stringify(value)) as T);
}
