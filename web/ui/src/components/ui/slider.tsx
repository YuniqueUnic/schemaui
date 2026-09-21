import * as React from "react";
import { Minus, Plus } from "lucide-react";
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
  /**
   * Reads typed text back to a number.
   *
   * The default accepts a plain number, a mark's own label, and a number
   * wearing a unit. Whatever `formatValue` prints has to be typeable back in,
   * or the readout would refuse its own output.
   */
  parseValue?: (text: string, marks: SliderMark[]) => number | null;
  /** Lets the readout be clicked and typed into. */
  editable?: boolean;
  /** Puts a −/+ pair beside the draft while editing, for one step at a time. */
  stepper?: boolean;
  /**
   * Names the quantity for screen readers, e.g. `"Opacity"`.
   *
   * Without it every slider on a page announces its readout as plain "value",
   * which tells a screen-reader user which control they are on only if there
   * happens to be exactly one.
   */
  valueLabel?: string;
  /** Shows the value but takes no input. */
  readOnly?: boolean;
  disabled?: boolean;
  className?: string;
  /** Passed to the first handle, so a field label can point at it. */
  id?: string;
}

/**
 * The window one handle may move in: the track's own ends, tightened by the
 * other handle when there is one.
 */
interface Window {
  min: number;
  step: number;
  low: number;
  high: number;
}

/** What happened to a number on its way onto the track. */
type Landing =
  | { value: number; moved: null }
  | { value: number; moved: "clamped" | "rounded" };

/**
 * Six decimals is where a step grid stops being meaningful and float noise
 * starts: `0.1 + 3 * 0.1` is `0.30000000000000004`, and a readout that prints
 * that has told the user something untrue about their own input.
 */
function round(value: number): number {
  return Number(value.toFixed(6));
}

/** The highest grid stop at or below `limit`. */
function stopBelow(limit: number, min: number, step: number): number {
  return round(min + Math.floor(round((limit - min) / step)) * step);
}

/** The lowest grid stop at or above `limit`. */
function stopAbove(limit: number, min: number, step: number): number {
  return round(min + Math.ceil(round((limit - min) / step)) * step);
}

/**
 * Where a raw number lands on the track.
 *
 * Every number lands somewhere: too low or too high clamps to the nearest end,
 * off-step rounds to the nearest stop. Refusing instead would leave the user
 * looking at a red box and an unchanged value, with nothing said about which
 * of the two rules they broke — and the track cannot represent what they typed
 * either way, so there is no reading of "reject" that keeps their number.
 */
function land(raw: number, window: Window): Landing {
  const clamped = Math.min(window.high, Math.max(window.low, raw));
  let value = round(window.min + Math.round((clamped - window.min) / window.step) * window.step);
  // Rounding to the nearest stop can step back out of the window the clamp
  // just put us in, so the ends are re-applied on the grid itself.
  if (value > window.high) value = stopBelow(window.high, window.min, window.step);
  if (value < window.low) value = stopAbove(window.low, window.min, window.step);

  if (value === round(raw)) return { value, moved: null };
  return { value, moved: clamped === raw ? "rounded" : "clamped" };
}

/** The first number inside a string, so `50%` and `f/1.8` read as 50 and 1.8. */
const NUMBER_IN_TEXT = /-?\d+(?:\.\d+)?(?:e[-+]?\d+)?/i;

/**
 * Read text as a number, in the order a person would expect it to work.
 *
 * A marked slider prints its stops as words (`medium`) or as numbers wearing a
 * unit (`50%`); both are what the user is looking at when they start typing,
 * so both have to read back.
 */
function defaultParseValue(text: string, marks: SliderMark[]): number | null {
  const trimmed = text.trim();
  if (!trimmed) return null;

  const plain = Number(trimmed);
  if (trimmed !== "" && Number.isFinite(plain)) return plain;

  const labelled = marks.find((mark) =>
    typeof mark.label === "string" &&
    mark.label.trim().toLowerCase() === trimmed.toLowerCase()
  );
  if (labelled) return labelled.value;

  const embedded = trimmed.match(NUMBER_IN_TEXT);
  if (embedded) {
    const num = Number(embedded[0]);
    if (Number.isFinite(num)) return num;
  }
  return null;
}

/** A line of feedback under the readout. */
interface Message {
  tone: "error" | "note";
  text: string;
}

/**
 * How wide the readout box has to be, in characters, to hold anything it will
 * ever print.
 *
 * Fixing the width is what stops the badge from jumping when it turns into an
 * input: an `<input>` left to itself is twenty characters wide regardless of
 * what it holds, so the readout would grow fivefold on the first click.
 */
/**
 * Visual width in half-width character units (monospace 1ch).
 * Full-width / CJK characters count as 2.
 */
function visualWidth(text: string): number {
  let count = 0;
  for (const ch of text) {
    const code = ch.charCodeAt(0);
    if (
      (code >= 0x1100 && code <= 0x115f) ||
      (code >= 0x2e80 && code <= 0xa4cf) ||
      (code >= 0xac00 && code <= 0xd7a3) ||
      (code >= 0xf900 && code <= 0xfaff) ||
      (code >= 0xfe10 && code <= 0xfe19) ||
      (code >= 0xfe30 && code <= 0xfe6f) ||
      (code >= 0xff00 && code <= 0xff60) ||
      (code >= 0xffe0 && code <= 0xffe6)
    ) {
      count += 2;
    } else {
      count += 1;
    }
  }
  return count;
}

/**
 * How wide the readout box has to be, in characters, to hold anything it will
 * ever print.
 *
 * Sizing generously with internal padding ensures text never clips or overflows
 * borders (preventing "[xxxx]x" overflow), while avoiding jarring layout shifts.
 */
function readoutChars(
  min: number,
  max: number,
  step: number,
  formatValue: (value: number) => string,
  marks: SliderMark[],
  currentValues: number[],
): number {
  const decimals = (String(step).split(".")[1] ?? "").length;
  const digits = Math.max(
    String(Math.trunc(min)).length,
    String(Math.trunc(max)).length,
  ) + (decimals > 0 ? decimals + 1 : 0);

  const printed = [
    formatValue(min),
    formatValue(max),
    ...currentValues.map(formatValue),
    ...marks.map((mark) => mark.label ?? formatValue(mark.value)),
  ].map(visualWidth);

  return Math.max(3, digits, ...printed);
}

const BADGE_BASE =
  "h-[22px] rounded-md border font-mono text-[11px] leading-[14px] tabular-nums transition-colors";

interface ReadoutProps {
  value: number;
  window: Window;
  marks: SliderMark[];
  formatValue: (value: number) => string;
  parseValue: (text: string, marks: SliderMark[]) => number | null;
  /** "Opacity", "Opacity minimum", … — already assembled by the caller. */
  name: string;
  maxChars: number;
  editable: boolean;
  /** Shows the −/+ pair beside the draft while editing; never while idle, so
   *  the badge itself stays a single, uncrowded target to click. */
  stepper: boolean;
  interactive: boolean;
  messageId: string;
  hasError: boolean;
  onMessage: (message: Message | null) => void;
  onCommit: (next: number) => void;
}

/**
 * The value badge, which doubles as its own input and stepper group.
 */
function Readout({
  value,
  window,
  marks,
  formatValue,
  parseValue,
  name,
  maxChars,
  editable,
  stepper,
  interactive,
  messageId,
  hasError,
  onMessage,
  onCommit,
}: ReadoutProps) {
  const [isEditing, setIsEditing] = React.useState(false);
  const [draft, setDraft] = React.useState("");
  const inputRef = React.useRef<HTMLInputElement>(null);
  const triggerRef = React.useRef<HTMLButtonElement>(null);
  const wasEditing = React.useRef(false);

  React.useEffect(() => {
    if (isEditing) {
      inputRef.current?.focus();
      inputRef.current?.select();
    } else if (wasEditing.current) {
      triggerRef.current?.focus();
    }
    wasEditing.current = isEditing;
  }, [isEditing]);

  const startEditing = () => {
    if (!interactive || !editable) return;
    setDraft(formatValue(value));
    onMessage(null);
    setIsEditing(true);
  };

  const commitDraft = () => {
    const parsed = parseValue(draft, marks);
    if (parsed === null || !Number.isFinite(parsed)) {
      onMessage({
        tone: "error",
        text: `Cannot read “${draft.trim()}” as a number`,
      });
      return false;
    }

    const landing = land(parsed, window);
    setIsEditing(false);
    onMessage(
      landing.moved === null ? null : {
        tone: "note",
        text: `${landing.moved === "clamped" ? "Clamped" : "Rounded"} to ${
          formatValue(landing.value)
        }`,
      },
    );
    if (landing.value !== value) onCommit(landing.value);
    return true;
  };

  const cancelEditing = (message: Message | null) => {
    setIsEditing(false);
    onMessage(message);
  };

  const stepInput = (direction: 1 | -1) => {
    const parsed = parseValue(draft, marks);
    const from = parsed === null || !Number.isFinite(parsed) ? value : parsed;
    const landing = land(round(from + direction * window.step), window);
    setDraft(formatValue(landing.value));
    onMessage(
      landing.moved === null ? null : {
        tone: "note",
        text: `${landing.moved === "clamped" ? "Clamped" : "Rounded"} to ${
          formatValue(landing.value)
        }`,
      },
    );
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      commitDraft();
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      cancelEditing(null);
    } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      stepInput(event.key === "ArrowUp" ? 1 : -1);
    }
  };

  const handleBlur = () => {
    if (commitDraft()) return;
    cancelEditing({ tone: "note", text: `Kept ${formatValue(value)}` });
  };

  const canEdit = interactive && editable;

  if (isEditing) {
    const activeChars = Math.max(maxChars, visualWidth(draft));
    const currentParsed = parseValue(draft, marks);
    const currentNumeric = currentParsed !== null && Number.isFinite(currentParsed)
      ? currentParsed
      : value;
    const canStepDown = interactive && currentNumeric > window.low;
    const canStepUp = interactive && currentNumeric < window.high;

    if (stepper) {
      return (
        <div
          style={{ width: `calc(${activeChars}ch + 4.25rem)`, minWidth: "5.5rem" }}
          className={cn(
            "flex h-[22px] items-stretch rounded-md border font-mono text-[11px] tabular-nums transition-colors",
            "bg-background text-foreground",
            hasError
              ? "border-destructive focus-within:ring-1 focus-within:ring-destructive"
              : "border-ring focus-within:ring-1 focus-within:ring-ring",
          )}
        >
          <button
            type="button"
            tabIndex={-1}
            aria-label={`Decrease ${name}`}
            disabled={!canStepDown}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => stepInput(-1)}
            className={cn(
              "flex w-6 shrink-0 items-center justify-center border-r border-border/60 text-muted-foreground transition-colors",
              !canStepDown
                ? "cursor-default opacity-40"
                : "cursor-pointer hover:bg-muted hover:text-foreground active:bg-muted/80",
            )}
          >
            <Minus className="h-3 w-3" aria-hidden="true" />
          </button>
          <input
            ref={inputRef}
            type="text"
            value={draft}
            inputMode={Number.isInteger(window.step) ? "numeric" : "decimal"}
            aria-label={`Edit ${name}`}
            aria-invalid={hasError}
            aria-describedby={messageId}
            onChange={(event) => {
              setDraft(event.target.value);
              if (hasError) onMessage(null);
            }}
            onKeyDown={handleKeyDown}
            onBlur={handleBlur}
            className="h-full min-w-0 flex-1 bg-transparent px-2 text-center font-mono text-[11px] leading-none tabular-nums outline-none"
          />
          <button
            type="button"
            tabIndex={-1}
            aria-label={`Increase ${name}`}
            disabled={!canStepUp}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => stepInput(1)}
            className={cn(
              "flex w-6 shrink-0 items-center justify-center border-l border-border/60 text-muted-foreground transition-colors",
              !canStepUp
                ? "cursor-default opacity-40"
                : "cursor-pointer hover:bg-muted hover:text-foreground active:bg-muted/80",
            )}
          >
            <Plus className="h-3 w-3" aria-hidden="true" />
          </button>
        </div>
      );
    }

    return (
      <input
        ref={inputRef}
        type="text"
        value={draft}
        style={{ width: `calc(${activeChars}ch + 2rem)`, minWidth: "3.5rem" }}
        inputMode={Number.isInteger(window.step) ? "numeric" : "decimal"}
        aria-label={`Edit ${name}`}
        aria-invalid={hasError}
        aria-describedby={messageId}
        onChange={(event) => {
          setDraft(event.target.value);
          if (hasError) onMessage(null);
        }}
        onKeyDown={handleKeyDown}
        onBlur={handleBlur}
        className={cn(
          BADGE_BASE,
          "border-ring bg-background px-2.5 py-0.5 text-center text-foreground outline-none focus:ring-1 focus:ring-ring",
          hasError && "border-destructive text-destructive focus:ring-destructive",
        )}
      />
    );
  }

  return (
    <button
      ref={triggerRef}
      type="button"
      onClick={startEditing}
      disabled={!canEdit}
      style={{ width: `calc(${maxChars}ch + 2rem)`, minWidth: "3.5rem" }}
      aria-label={canEdit ? `Edit ${name}` : name}
      className={cn(
        BADGE_BASE,
        "border-border bg-muted/60 px-2.5 py-0.5 text-center text-foreground",
        canEdit
          ? "cursor-pointer hover:border-ring/50 hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          : "cursor-default opacity-80",
      )}
    >
      {formatValue(value)}
    </button>
  );
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
  parseValue = defaultParseValue,
  editable = false,
  stepper = false,
  valueLabel,
  readOnly = false,
  disabled,
  className,
  id,
}: SliderProps) {
  const [message, setMessage] = React.useState<Message | null>(null);
  const messageId = React.useId();

  const effectiveStep = step !== undefined && step > 0 ? step : 1;
  const span = max - min || 1;
  const percent = (current: number) =>
    Math.min(100, Math.max(0, ((current - min) / span) * 100));

  const isRange = value.length > 1;
  const [low, high] = isRange ? value : [min, value[0]];
  const interactive = !disabled && !readOnly;

  // Hold the gap here rather than in the caller: the caller only sees the
  // settled pair, and by then the crossing has already happened. Both the drag
  // path and the typed path read this, so the two cannot disagree about where
  // a handle is allowed to stop.
  const windowFor = (index: number): Window => {
    if (!isRange) return { min, step: effectiveStep, low: min, high: max };
    const gap = effectiveStep * Math.max(1, minStepsBetweenThumbs);
    return index === 0
      ? { min, step: effectiveStep, low: min, high: round(value[1] - gap) }
      : { min, step: effectiveStep, low: round(value[0] + gap), high: max };
  };

  const commit = (index: number, next: number) => {
    const updated = [...value];
    updated[index] = land(next, windowFor(index)).value;
    onValueChange(updated);
  };

  const maxChars = readoutChars(min, max, effectiveStep, formatValue, marks, value);
  const nameFor = (index: number) => {
    const noun = valueLabel ?? "value";
    if (!isRange) return noun;
    return valueLabel
      ? `${valueLabel} ${index === 0 ? "minimum" : "maximum"}`
      : `${index === 0 ? "minimum" : "maximum"} value`;
  };

  const readoutFor = (index: number) => {
    const window = windowFor(index);
    return (
      <Readout
        key={index}
        value={value[index]}
        window={window}
        marks={marks}
        formatValue={formatValue}
        parseValue={parseValue}
        name={nameFor(index)}
        maxChars={maxChars}
        editable={editable}
        stepper={stepper}
        interactive={interactive}
        messageId={messageId}
        hasError={message?.tone === "error"}
        onMessage={setMessage}
        onCommit={(next) => commit(index, next)}
      />
    );
  };

  return (
    <div className={cn("w-full", className)}>
      {showValue && (
        <div className="mb-2 flex flex-col items-end gap-0.5">
          {editable || stepper
            ? (
              <div className="flex items-center gap-2">
                {isRange ? (
                  <>
                    {readoutFor(0)}
                    <span
                      aria-hidden="true"
                      className="select-none font-mono text-xs text-muted-foreground/60"
                    >
                      –
                    </span>
                    {readoutFor(1)}
                  </>
                ) : (
                  readoutFor(0)
                )}
              </div>
            )
            : (
              <span className="rounded-md border border-border bg-muted/60 px-2.5 py-0.5 font-mono text-[11px] tabular-nums text-foreground">
                {value.map(formatValue).join(" – ")}
              </span>
            )}
          {/* The row is always here, empty or not: a message that appears out
              of nowhere would push the track down as the user reads it. */}
          <span
            id={messageId}
            role={message?.tone === "error" ? "alert" : "status"}
            className={cn(
              "h-3.5 font-mono text-[10px] leading-[14px]",
              message?.tone === "error"
                ? "text-destructive"
                : "text-muted-foreground",
            )}
          >
            {message?.text ?? ""}
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
            step={effectiveStep}
            value={current}
            disabled={disabled || readOnly}
            aria-label={isRange
              ? (index === 0 ? "Minimum" : "Maximum")
              : undefined}
            onChange={(event) => {
              setMessage(null);
              commit(index, Number(event.target.value));
            }}
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
