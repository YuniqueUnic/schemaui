import * as React from "react";
import { cn } from "@/lib/utils";

/** A labelled stop along the track. */
export interface SliderMark {
  value: number;
  label?: string | null;
}

export interface SliderProps {
  value: number[];
  onValueChange(value: number[]): void;
  min: number;
  max: number;
  step?: number;
  /** Labelled stops drawn under the track. */
  marks?: SliderMark[];
  /**
   * Minimum number of steps that must stay between two handles.
   *
   * Native range inputs know nothing about each other, so without this a drag
   * would let the low handle slide past the high one and silently swap which
   * value is which. Defaults to 1 step, i.e. the handles may touch but not
   * cross.
   */
  minStepsBetweenThumbs?: number;
  /** Adds a readout above the track. */
  showValue?: boolean;
  /** How the readout prints a number. Defaults to a plain string. */
  formatValue?: (value: number) => string;
  disabled?: boolean;
  className?: string;
  /** Passed to the first handle, so a field label can point at it. */
  id?: string;
}

/**
 * Slider on native range inputs.
 *
 * One handle is one `<input type="range">`; two handles are two inputs stacked
 * on a shared track. The browser keeps keyboard stepping, touch dragging,
 * screen-reader announcements and form semantics, and this component only
 * supplies the look — which is why there is no pointer maths here at all.
 *
 * The visible track is drawn once, underneath, so a two-handle slider shows a
 * single strip with the chosen span filled rather than two overlapping tracks.
 */
export function Slider({
  value,
  onValueChange,
  min,
  max,
  step,
  marks = [],
  minStepsBetweenThumbs = 1,
  showValue = false,
  formatValue = (current) => String(current),
  disabled,
  className,
  id,
}: SliderProps) {
  const span = max - min || 1;
  const percent = (current: number) =>
    Math.min(100, Math.max(0, ((current - min) / span) * 100));

  const isRange = value.length > 1;
  const [low, high] = isRange ? value : [min, value[0]];

  const commit = (index: number, next: number) => {
    const updated = [...value];
    if (isRange) {
      // Hold the gap here rather than in the caller: the caller only sees the
      // settled pair, and by then the crossing has already happened.
      const gap = (step ?? 1) * Math.max(1, minStepsBetweenThumbs);
      const other = updated[index === 0 ? 1 : 0];
      const bounded = index === 0
        ? Math.min(next, other - gap)
        : Math.max(next, other + gap);
      // The outer clamp is defensive — a native range input already refuses to
      // report a value outside its own min/max — but the gap arithmetic above
      // could in principle push past an end, and a value the track cannot
      // represent would desynchronise the fill from the handle.
      updated[index] = Math.min(max, Math.max(min, bounded));
    } else {
      updated[index] = next;
    }
    onValueChange(updated);
  };

  return (
    <div className={cn("w-full", className)}>
      {showValue && (
        <div className="mb-2 flex justify-end">
          <span className="rounded-md border border-border bg-muted/60 px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-foreground">
            {value.map(formatValue).join(" – ")}
          </span>
        </div>
      )}

      <div className="relative flex h-5 items-center">
        {
          /* The one visible track. Its fill is the chosen span, so a range reads
            as an interval rather than as two unrelated positions. */
        }
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-0 h-1.5 rounded-full bg-secondary"
        >
          <div
            className="absolute h-full rounded-full bg-primary"
            style={{
              left: `${percent(low)}%`,
              right: `${100 - percent(high)}%`,
            }}
          />
        </div>

        {value.map((current, index) => (
          <input
            key={index}
            id={index === 0 ? id : undefined}
            type="range"
            className={cn(
              "range-input",
              isRange && "range-input-stacked",
            )}
            min={min}
            max={max}
            step={step ?? 1}
            value={current}
            disabled={disabled}
            aria-label={isRange
              ? (index === 0 ? "Minimum" : "Maximum")
              : undefined}
            onChange={(event) => commit(index, Number(event.target.value))}
            // A single-handle slider fills from the left edge; a stacked pair
            // would each draw their own fill, so they get a transparent track
            // from the stylesheet instead.
            style={isRange
              ? undefined
              : ({ "--fill": `${percent(current)}%` } as React.CSSProperties)}
          />
        ))}
      </div>

      {marks.length > 0 && (
        <div className="relative mt-1.5 h-3.5" aria-hidden="true">
          {marks.map((mark) => (
            <span
              key={mark.value}
              className="absolute top-0 -translate-x-1/2 whitespace-nowrap font-mono text-[10px] leading-none text-muted-foreground"
              style={{ left: `${percent(mark.value)}%` }}
            >
              {mark.label ?? formatValue(mark.value)}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
