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
  /** Parses user input string back to a numeric value. */
  parseValue?: (text: string) => number | null;
  /** Enables direct text editing of the value display. */
  editable?: boolean;
  /** Disables editing while retaining normal appearance. */
  readOnly?: boolean;
  disabled?: boolean;
  className?: string;
  /** Passed to the first handle, so a field label can point at it. */
  id?: string;
}

interface EditableValueProps {
  index: number;
  value: number;
  allValues: number[];
  min: number;
  max: number;
  step: number;
  minStepsBetweenThumbs: number;
  isRange: boolean;
  formatValue: (value: number) => string;
  parseValue: (text: string) => number | null;
  onCommit: (index: number, next: number) => void;
  disabled?: boolean;
  readOnly?: boolean;
}

function EditableValue({
  index,
  value,
  allValues,
  min,
  max,
  step,
  minStepsBetweenThumbs,
  isRange,
  formatValue,
  parseValue,
  onCommit,
  disabled,
  readOnly,
}: EditableValueProps) {
  const [isEditing, setIsEditing] = React.useState(false);
  const [draft, setDraft] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);
  const inputRef = React.useRef<HTMLInputElement>(null);
  const errorId = React.useId();

  const accessibleLabel = isRange
    ? index === 0
      ? "Edit minimum value"
      : "Edit maximum value"
    : "Edit value";

  const startEditing = () => {
    if (disabled || readOnly) return;
    setDraft(formatValue(value));
    setError(null);
    setIsEditing(true);
  };

  React.useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isEditing]);

  const validate = (text: string): { ok: true; val: number } | { ok: false; message: string } => {
    const parsed = parseValue(text);
    if (parsed === null || !Number.isFinite(parsed)) {
      return { ok: false, message: "Enter a valid number" };
    }
    if (parsed < min) {
      return { ok: false, message: `Value must be at least ${min}` };
    }
    if (parsed > max) {
      return { ok: false, message: `Value must be at most ${max}` };
    }

    // Step validation with floating-point tolerance
    const steps = Math.round((parsed - min) / step);
    const snapped = min + steps * step;
    if (Math.abs(parsed - snapped) > 1e-6) {
      return { ok: false, message: `Value must be a multiple of ${step}` };
    }

    if (isRange) {
      const gap = step * Math.max(1, minStepsBetweenThumbs);
      if (index === 0) {
        const high = allValues[1];
        if (parsed > high - gap) {
          return {
            ok: false,
            message: `Minimum cannot exceed ${high - gap}`,
          };
        }
      } else {
        const low = allValues[0];
        if (parsed < low + gap) {
          return {
            ok: false,
            message: `Maximum must be at least ${low + gap}`,
          };
        }
      }
    }

    return { ok: true, val: snapped };
  };

  const handleCommit = () => {
    const result = validate(draft);
    if (result.ok) {
      onCommit(index, result.val);
      setIsEditing(false);
      setError(null);
    } else {
      setError(result.message);
    }
  };

  const handleCancel = () => {
    setIsEditing(false);
    setError(null);
    setDraft(formatValue(value));
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleCommit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      handleCancel();
    }
  };

  const handleBlur = () => {
    const result = validate(draft);
    if (result.ok) {
      onCommit(index, result.val);
      setIsEditing(false);
      setError(null);
    } else {
      // Invalid input on blur cancels to avoid committing an invalid slider state
      handleCancel();
    }
  };

  if (isEditing) {
    const isIntegerStep = Number.isInteger(step);
    return (
      <div className="relative inline-flex items-center">
        <input
          ref={inputRef}
          type="text"
          value={draft}
          inputMode={isIntegerStep ? "numeric" : "decimal"}
          aria-label={accessibleLabel}
          aria-invalid={Boolean(error)}
          aria-describedby={error ? errorId : undefined}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
            setDraft(e.target.value);
            if (error) setError(null);
          }}
          onKeyDown={handleKeyDown}
          onBlur={handleBlur}
          className={cn(
            "h-[22px] min-w-[3rem] rounded-md border bg-background px-1.5 py-0.5 text-center font-mono text-[11px] tabular-nums text-foreground outline-none transition-colors",
            error
              ? "border-destructive text-destructive focus:border-destructive focus:ring-1 focus:ring-destructive"
              : "border-ring focus:border-ring focus:ring-1 focus:ring-ring",
          )}
        />
        {error && (
          <span id={errorId} role="alert" className="sr-only">
            {error}
          </span>
        )}
      </div>
    );
  }

  const canEdit = !disabled && !readOnly;

  return (
    <button
      type="button"
      onClick={startEditing}
      disabled={!canEdit}
      aria-label={accessibleLabel}
      className={cn(
        "rounded-md border border-border bg-muted/60 px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-foreground transition-colors",
        canEdit
          ? "cursor-pointer hover:bg-muted hover:border-ring/50 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          : "cursor-default opacity-80",
      )}
    >
      {formatValue(value)}
    </button>
  );
}

const defaultParseValue = (text: string): number | null => {
  const trimmed = text.trim();
  if (!trimmed) return null;
  const num = Number(trimmed);
  return Number.isFinite(num) ? num : null;
};

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
  step = 1,
  marks = [],
  minStepsBetweenThumbs = 1,
  showValue = false,
  formatValue = (current) => String(current),
  parseValue = defaultParseValue,
  editable = false,
  readOnly = false,
  disabled,
  className,
  id,
}: SliderProps) {
  const effectiveStep = step > 0 ? step : 1;
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
      const gap = effectiveStep * Math.max(1, minStepsBetweenThumbs);
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
      updated[index] = Math.min(max, Math.max(min, next));
    }
    onValueChange(updated);
  };

  return (
    <div className={cn("w-full", className)}>
      {showValue && (
        <div className="mb-2 flex min-h-[22px] items-center justify-end">
          {editable ? (
            isRange ? (
              <div className="flex items-center gap-1">
                <EditableValue
                  index={0}
                  value={value[0]}
                  allValues={value}
                  min={min}
                  max={max}
                  step={effectiveStep}
                  minStepsBetweenThumbs={minStepsBetweenThumbs}
                  isRange={true}
                  formatValue={formatValue}
                  parseValue={parseValue}
                  onCommit={commit}
                  disabled={disabled}
                  readOnly={readOnly}
                />
                <span
                  aria-hidden="true"
                  className="select-none font-mono text-[11px] text-muted-foreground"
                >
                  –
                </span>
                <EditableValue
                  index={1}
                  value={value[1]}
                  allValues={value}
                  min={min}
                  max={max}
                  step={effectiveStep}
                  minStepsBetweenThumbs={minStepsBetweenThumbs}
                  isRange={true}
                  formatValue={formatValue}
                  parseValue={parseValue}
                  onCommit={commit}
                  disabled={disabled}
                  readOnly={readOnly}
                />
              </div>
            ) : (
              <EditableValue
                index={0}
                value={value[0]}
                allValues={value}
                min={min}
                max={max}
                step={effectiveStep}
                minStepsBetweenThumbs={minStepsBetweenThumbs}
                isRange={false}
                formatValue={formatValue}
                parseValue={parseValue}
                onCommit={commit}
                disabled={disabled}
                readOnly={readOnly}
              />
            )
          ) : (
            <span className="rounded-md border border-border bg-muted/60 px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-foreground">
              {value.map(formatValue).join(" – ")}
            </span>
          )}
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
            step={effectiveStep}
            value={current}
            disabled={disabled || readOnly}
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
