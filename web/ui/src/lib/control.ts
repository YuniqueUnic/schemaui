import type { FieldBounds, FieldControl, ScalarKind, UiNode } from "../types";

/**
 * The control actually used to render a node.
 *
 * `number` is not part of the schema vocabulary — it is the plain numeric
 * input, which is what a number falls back to when no slider can be drawn.
 */
export type ResolvedControl = FieldControl | "number";

/** A slider can only be drawn when both ends of its track are known. */
export interface ResolvedBounds {
  min: number;
  max: number;
  step: number;
  marks: Array<{ value: number; label?: string | null }>;
}

/**
 * Pick the control for a node.
 *
 * The schema's `x-control` wins when it names something this build can draw.
 * Everything else falls back to the default for the node's shape, in this
 * order of preference:
 *
 * 1. the requested control, if it fits the value;
 * 2. the default for that shape (`select` for an enum, `switch` for a boolean,
 *    `textarea` for a multi-line string, …);
 * 3. a plain input.
 *
 * Falling back rather than failing matters because the two sides version
 * independently: a schema may name a control a given build has never heard of,
 * and the right answer there is a usable form, not an empty card.
 */
export function resolveControl(node: UiNode): ResolvedControl {
  const requested = node.control ?? null;
  const fallback = defaultControl(node);

  if (requested && fits(requested, node)) {
    return requested;
  }
  return fallback;
}

/** The control a node gets when the schema does not ask for one. */
export function defaultControl(node: UiNode): ResolvedControl {
  if (node.kind.type === "array") {
    return "number";
  }
  if (node.kind.type !== "field") {
    return "number";
  }
  if (node.kind.enum_options?.length) {
    return "select";
  }
  switch (node.kind.scalar) {
    case "boolean":
      return "switch";
    case "integer":
    case "number":
      return "number";
    case "string":
    default:
      return node.kind.multiline ? "textarea" : "text";
  }
}

/**
 * Whether this build can actually draw `control` for this node.
 *
 * Sliders and ranges additionally need bounds, because a track with only one
 * end would require inventing the other — and an invented maximum is a worse
 * answer than a number box.
 */
function fits(control: FieldControl, node: UiNode): boolean {
  // Destructured rather than tested through a boolean flag: TypeScript only
  // narrows a union on the discriminant itself, so `const isField = ...` would
  // leave `node.kind.scalar` unreachable.
  const kind = node.kind;
  const scalar = kind.type === "field" ? kind.scalar : null;
  const hasEnum = kind.type === "field" && Boolean(kind.enum_options?.length);

  switch (control) {
    case "text":
    case "textarea":
    case "mermaid":
      return scalar === "string";
    case "color":
      return scalar === "string";
    case "switch":
    case "checkbox":
      return scalar === "boolean";
    case "slider":
      return (scalar === "number" || scalar === "integer") &&
        resolveBounds(node) !== null;
    case "select":
    case "segmented":
    case "radio":
      return hasEnum;
    case "range": {
      // The pair shape is re-checked here rather than trusted: the parser
      // rejects a mismatched `range`, but this build can also be handed a
      // snapshot compiled by a different version.
      const kind = node.kind;
      return (
        kind.type === "array" &&
        kind.min_items === 2 &&
        kind.max_items === 2 &&
        resolveBounds(node) !== null
      );
    }
    default:
      return false;
  }
}

/**
 * Read a usable `[min, max, step]` off a node, or `null` when either end is
 * missing or they do not form an interval.
 *
 * The step is only guessed when the schema does not declare one, and the guess
 * depends on what the value is: an integer steps by whole units, because a
 * finer step would offer values the schema rejects. A `number` gets a step that
 * divides the span into roughly a hundred stops, snapped to a round magnitude —
 * so 0–1 steps by 0.01 and 0–100 by 1, rather than a fixed 1 that would turn a
 * unit interval into a two-position toggle.
 */
export function resolveBounds(node: UiNode): ResolvedBounds | null {
  const bounds: FieldBounds | null | undefined = node.bounds;
  const min = bounds?.minimum;
  const max = bounds?.maximum;
  if (typeof min !== "number" || typeof max !== "number" || max <= min) {
    return null;
  }

  const declared = bounds?.step;
  // An integer range (`"range"` on a two-number array) is an *array* of integer
  // items, not an integer field — so the item kind has to be asked, not the
  // node kind. Asking only the node kind made every range fall through to the
  // fractional guess, and integer sliders produced values like 4.6.
  const isInteger = scalarOf(node) === "integer";
  const step = typeof declared === "number" && declared > 0
    ? declared
    : isInteger
    ? 1
    : roundStep((max - min) / 100);

  return { min, max, step, marks: bounds?.marks ?? [] };
}

/**
 * The scalar a node's value holds: the field's own scalar, or the item's scalar
 * when the node is the two-number array a `range` edits.
 */
function scalarOf(node: UiNode): ScalarKind | null {
  const kind = node.kind;
  if (kind.type === "field") return kind.scalar;
  if (kind.type === "array") {
    return kind.item.type === "field" ? kind.item.scalar : null;
  }
  return null;
}

/**
 * Round a derived step down to something a person would choose, so a 0–1 range
 * steps by 0.01 rather than 0.009999.
 */
function roundStep(raw: number): number {
  if (raw <= 0) return 1;
  const magnitude = Math.pow(10, Math.floor(Math.log10(raw)));
  return Math.max(magnitude, Math.round(raw / magnitude) * magnitude);
}

/** Clamp a value into a resolved range and snap it to the step. */
export function snapToBounds(value: number, bounds: ResolvedBounds): number {
  const clamped = Math.min(bounds.max, Math.max(bounds.min, value));
  const steps = Math.round((clamped - bounds.min) / bounds.step);
  const snapped = bounds.min + steps * bounds.step;
  // Re-round: 0.1 + 3 * 0.1 lands on 0.30000000000000004.
  return Number(snapped.toFixed(6));
}

/** Where a value sits along its track, as a percentage. */
export function trackPercent(value: number, bounds: ResolvedBounds): number {
  return ((value - bounds.min) / (bounds.max - bounds.min)) * 100;
}
