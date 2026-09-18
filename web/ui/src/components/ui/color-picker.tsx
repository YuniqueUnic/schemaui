import * as React from "react";
import { Check } from "lucide-react";
import { cn } from "@/lib/utils";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";
import {
  hexToHsv,
  type Hsv,
  hsvToHex,
  isColor,
  readableInk,
} from "@/lib/color";

/**
 * A palette that stays useful against the monochrome theme: a neutral ramp for
 * the many cases where a colour is really a shade, then the few hues that
 * actually get picked.
 */
const SWATCHES = [
  "#000000",
  "#404040",
  "#737373",
  "#a3a3a3",
  "#d4d4d4",
  "#ffffff",
  "#ef4444",
  "#f97316",
  "#eab308",
  "#22c55e",
  "#06b6d4",
  "#3b82f6",
  "#6366f1",
  "#a855f7",
  "#ec4899",
];

interface ColorPickerProps {
  value: string;
  onChange(value: string): void;
  /** Passed to the trigger so a field label can point at it. */
  id?: string;
  disabled?: boolean;
}

/**
 * Colour picker: swatch trigger, saturation/brightness square, hue track,
 * hex entry and a preset row.
 *
 * The hex input is the accessible route to every value — the square and the
 * hue track are pointer enhancements and are hidden from assistive tech rather
 * than half-described, because "saturation 40%, brightness 80%" is not a
 * sentence that helps anyone pick a colour.
 */
export function ColorPicker(
  { value, onChange, id, disabled }: ColorPickerProps,
) {
  const [open, setOpen] = React.useState(false);

  // Hue is kept locally: a fully desaturated colour is grey, and grey has no
  // hue to recover, so re-deriving it from the value would snap the handle back
  // to red the moment someone dragged saturation to zero.
  const [hsv, setHsv] = React.useState<Hsv>(
    () => hexToHsv(value) ?? { h: 0, s: 0, v: 0 },
  );

  // `null` means "showing the committed value"; a string means the user is
  // mid-edit and their text is authoritative, even while it is still invalid.
  const [draft, setDraft] = React.useState<string | null>(null);
  const shown = draft ?? value;

  // The last value this component handed to the parent. Used to tell our own
  // value coming back from an edit made elsewhere.
  const emitted = React.useRef<string | null>(null);

  React.useEffect(() => {
    // Ignore the echo of our own commit. Without this the effect would fire on
    // the render *before* the parent has applied the change, read the stale
    // prop, and reset the hue to whatever that old colour had — so dragging
    // saturation to zero would lose the hue on the way back out.
    if (value === emitted.current) return;
    const parsed = hexToHsv(value);
    if (!parsed) return;
    setHsv(parsed);
  }, [value]);

  const commit = (next: Hsv) => {
    const hex = hsvToHex(next);
    setHsv(next);
    emitted.current = hex;
    onChange(hex);
  };

  const preview = isColor(value) ? value : "#ffffff";

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        id={id}
        disabled={disabled}
        aria-label={`Colour ${value || "unset"}`}
        className={cn(
          "inline-flex h-9 w-full items-center gap-2 rounded-md border border-input bg-background px-2 text-left text-sm",
          "shadow-sm transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
          "disabled:cursor-not-allowed disabled:opacity-50",
        )}
      >
        <span
          className="h-5 w-5 shrink-0 rounded border border-border"
          style={{ backgroundColor: preview }}
        />
        <span className="truncate font-mono text-xs text-foreground">
          {value || "—"}
        </span>
      </PopoverTrigger>

      <PopoverContent className="w-60">
        <SaturationArea hsv={hsv} onChange={commit} />

        <div className="mt-3">
          <HueTrack
            hue={hsv.h}
            onChange={(h) => commit({ ...hsv, h })}
          />
        </div>

        <div className="mt-3 flex items-center gap-2">
          <span
            className="h-8 w-8 shrink-0 rounded border border-border"
            style={{ backgroundColor: preview }}
          />
          <div className="relative flex-1">
            <span className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 font-mono text-xs text-muted-foreground">
              #
            </span>
            <input
              value={shown.replace(/^#/, "")}
              spellCheck={false}
              aria-label="Hex colour"
              onChange={(event) => {
                const next = event.target.value;
                setDraft(next);
                if (!isColor(next)) return;
                const hex = `#${next.replace(/^#/, "")}`;
                // Typed hex sets the hue as well as the colour: unlike a drag
                // inside the square, this is the user naming a colour outright,
                // so there is no hue worth preserving over it.
                const parsed = hexToHsv(hex);
                if (parsed) setHsv(parsed);
                emitted.current = hex;
                onChange(hex);
              }}
              onBlur={() => setDraft(null)}
              className="h-8 w-full rounded-md border border-input bg-background pl-6 pr-2 font-mono text-xs uppercase text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
          </div>
        </div>

        <div className="mt-3 grid grid-cols-8 gap-1.5">
          {SWATCHES.map((swatch) => {
            const active = swatch.toLowerCase() === value.toLowerCase();
            return (
              <button
                key={swatch}
                type="button"
                title={swatch}
                aria-label={swatch}
                onClick={() => {
                  const parsed = hexToHsv(swatch);
                  if (parsed) commit(parsed);
                }}
                className={cn(
                  "flex h-5 w-full items-center justify-center rounded border transition-transform hover:scale-110",
                  active ? "border-foreground" : "border-border",
                )}
                style={{ backgroundColor: swatch }}
              >
                {active && (
                  <Check
                    className="h-3 w-3"
                    style={{ color: readableInk(swatch) }}
                  />
                )}
              </button>
            );
          })}
        </div>
      </PopoverContent>
    </Popover>
  );
}

/** The saturation (x) / brightness (y) square. */
function SaturationArea({
  hsv,
  onChange,
}: {
  hsv: Hsv;
  onChange(next: Hsv): void;
}) {
  const area = usePointerArea((x, y) => onChange({ ...hsv, s: x, v: 1 - y }));

  return (
    <div
      {...area}
      aria-hidden="true"
      className="relative h-32 w-full cursor-crosshair touch-none rounded-md border border-border"
      style={{
        backgroundColor: hsvToHex({ h: hsv.h, s: 1, v: 1 }),
        backgroundImage:
          "linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, transparent)",
      }}
    >
      <span
        className="pointer-events-none absolute h-3.5 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.4)]"
        style={{
          left: `${hsv.s * 100}%`,
          top: `${(1 - hsv.v) * 100}%`,
        }}
      />
    </div>
  );
}

/** The hue track, drawn as the spectrum it selects from. */
function HueTrack({
  hue,
  onChange,
}: {
  hue: number;
  onChange(hue: number): void;
}) {
  const track = usePointerArea((x) => onChange(Math.round(x * 360)));

  return (
    <div
      {...track}
      aria-hidden="true"
      className="relative h-3 w-full cursor-pointer touch-none rounded-full border border-border"
      style={{
        backgroundImage:
          "linear-gradient(to right, #f00 0%, #ff0 17%, #0f0 33%, #0ff 50%, #00f 67%, #f0f 83%, #f00 100%)",
      }}
    >
      <span
        className="pointer-events-none absolute top-1/2 h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.4)]"
        style={{ left: `${(hue / 360) * 100}%` }}
      />
    </div>
  );
}

/**
 * Pointer handling shared by both surfaces: normalised `(x, y)` in 0–1,
 * captured so a drag keeps tracking once the pointer leaves the element.
 */
function usePointerArea(onMove: (x: number, y: number) => void) {
  const read = (element: HTMLElement, event: React.PointerEvent) => {
    const rect = element.getBoundingClientRect();
    const clamp = (value: number) => Math.min(1, Math.max(0, value));
    return {
      x: clamp((event.clientX - rect.left) / rect.width),
      y: clamp((event.clientY - rect.top) / rect.height),
    };
  };

  return {
    onPointerDown: (event: React.PointerEvent<HTMLElement>) => {
      event.currentTarget.setPointerCapture(event.pointerId);
      const { x, y } = read(event.currentTarget, event);
      onMove(x, y);
    },
    onPointerMove: (event: React.PointerEvent<HTMLElement>) => {
      if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
      const { x, y } = read(event.currentTarget, event);
      onMove(x, y);
    },
  };
}
