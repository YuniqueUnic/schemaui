import type { JsonValue, UiNode } from "../../../types";
import { Slider } from "../../ui/slider";
import { resolveBounds, snapToBounds } from "../../../lib/control";

interface RangeControlProps {
  node: UiNode;
  value: JsonValue | undefined;
  onChange: (pointer: string, value: JsonValue) => void;
}

/**
 * A pair of numbers rendered as one track with two handles.
 *
 * Rendered in place of the list editor rather than beside it: a two-element
 * array of numbers *is* an interval, and showing it as a list with an "add
 * item" button invites the user to break the shape the schema requires.
 *
 * The bounds come from the schema; without both ends this control is not
 * offered at all (see `resolveControl`), because a half-open interval has no
 * track to draw.
 */
export function RangeControl({ node, value, onChange }: RangeControlProps) {
  const bounds = resolveBounds(node);
  if (!bounds) return null;

  const current = readPair(value, bounds.min, bounds.max);

  const commit = (next: number[]) => {
    const [low, high] = [...next].sort((a, b) => a - b);
    onChange(node.pointer, [
      snapToBounds(low, bounds),
      snapToBounds(high, bounds),
    ]);
  };

  return (
    <div className="space-y-1">
      <Slider
        min={bounds.min}
        max={bounds.max}
        step={bounds.step}
        marks={bounds.marks}
        value={current}
        // The pair prints as one badge above the track, the same place the
        // single-value slider puts its readout.
        showValue
        // The slider keeps the handles one step apart, so a drag cannot cross
        // them; the sort in `commit` is for values that arrive out of order
        // from the document rather than from a drag.
        minStepsBetweenThumbs={1}
        onValueChange={commit}
      />
      {/* Only when there are no marks. Marks print the ends of the track
          themselves, so adding them here as well would label 0 and 2000 twice
          on consecutive rows. */}
      {bounds.marks.length === 0 && (
        <div className="flex items-center justify-between font-mono text-[11px] tabular-nums text-muted-foreground">
          <span>{bounds.min}</span>
          <span>{bounds.max}</span>
        </div>
      )}
    </div>
  );
}

/**
 * Read a `[low, high]` pair, tolerating anything the document happens to hold.
 *
 * A stored value can be the wrong length or the wrong type — the form is not
 * the only writer — and a range that renders as `NaN – NaN` tells the user
 * nothing about what to fix.
 */
function readPair(
  value: JsonValue | undefined,
  min: number,
  max: number,
): number[] {
  if (!Array.isArray(value) || value.length !== 2) {
    return [min, max];
  }
  const [low, high] = value;
  if (typeof low !== "number" || typeof high !== "number") {
    return [min, max];
  }
  return [Math.min(low, high), Math.max(low, high)];
}
