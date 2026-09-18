import { useState } from "react";
import {
  AlertCircle,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  Keyboard,
  LoaderCircle,
  LocateFixed,
  Save,
} from "lucide-react";
import { cn } from "@/lib/utils";

interface ShortcutHint {
  combo: string;
  label: string;
}

interface StatusBarProps {
  status: string;
  dirty: boolean;
  validating: boolean;
  saving?: boolean;
  exiting?: boolean;
  errorCount: number;
  focusLabel?: string;
  shortcuts?: ShortcutHint[];
  onErrorsClick?: () => void;
}

const DEFAULT_SHORTCUTS: ShortcutHint[] = [
  { combo: "Ctrl / Cmd + S", label: "Save changes" },
];

type StatusTone = "ready" | "dirty" | "error" | "busy";

interface StatusModel {
  tone: StatusTone;
  badge: string;
  message: string;
}

function buildStatusModel({
  status,
  dirty,
  validating,
  saving,
  exiting,
  errorCount,
}: Pick<
  StatusBarProps,
  "status" | "dirty" | "validating" | "saving" | "exiting" | "errorCount"
>): StatusModel {
  if (exiting) {
    return { tone: "busy", badge: "Exiting", message: "Ending session…" };
  }
  if (saving) {
    return { tone: "busy", badge: "Saving", message: "Persisting changes…" };
  }
  if (validating) {
    return {
      tone: "busy",
      badge: "Validating",
      message: "Checking the current document…",
    };
  }
  if (errorCount > 0) {
    return {
      tone: "error",
      badge: `Errors ${errorCount}`,
      message: compactStatus(status, "Fix validation errors before saving."),
    };
  }
  if (dirty) {
    return {
      tone: "dirty",
      badge: "Unsaved",
      message: compactStatus(status, "Changes are staged locally."),
    };
  }
  return {
    tone: "ready",
    badge: "Ready",
    message: compactStatus(status, "Everything is synced."),
  };
}

function compactStatus(status: string, fallback: string) {
  const trimmed = status.trim();
  if (!trimmed || trimmed === "Ready") {
    return fallback;
  }
  return trimmed;
}

/**
 * Tones on the preset's monochrome scale.
 *
 * The theme carries no hue, so state is carried by *weight* instead: a filled
 * pill means "this wants attention" (unsaved), an outlined one means "nothing
 * to do", and red is reserved for the single case that has earned a colour —
 * errors. A green "Ready" and a blue "Saving" would each be a second accent
 * competing with the content for the same glance.
 */
const TONE_CLASS: Record<StatusTone, string> = {
  ready: "border-border bg-transparent text-muted-foreground",
  dirty: "border-transparent bg-primary text-primary-foreground",
  error: "border-destructive/40 bg-destructive/10 text-destructive",
  busy: "border-border bg-muted text-muted-foreground",
};

/**
 * The one status line for the whole app.
 *
 * Everything the session can be doing — ready, unsaved, saving, validating,
 * exiting — collapses into a single tone-coloured pill, because they are
 * mutually exclusive and a row of badges saying the same thing in different
 * words is what made this bar noisy. The keyboard reference stays folded away
 * behind the chevron: it is a reference, not a status.
 */
export function StatusBar({
  status,
  dirty,
  validating,
  saving = false,
  exiting = false,
  errorCount,
  focusLabel,
  shortcuts = DEFAULT_SHORTCUTS,
  onErrorsClick,
}: StatusBarProps) {
  const [expanded, setExpanded] = useState(false);

  const statusModel = buildStatusModel({
    status,
    dirty,
    validating,
    saving,
    exiting,
    errorCount,
  });

  const toneClass = TONE_CLASS[statusModel.tone];

  const statusIcon = statusModel.tone === "error"
    ? <AlertCircle className="h-3.5 w-3.5" />
    : statusModel.tone === "busy"
    ? <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
    : statusModel.tone === "dirty"
    ? <Save className="h-3.5 w-3.5" />
    : <CheckCircle2 className="h-3.5 w-3.5" />;

  return (
    <footer className="shrink-0 border-t border-border/60 bg-background px-3 py-1.5 text-xs md:px-4 lg:px-6">
      <div className="flex items-center gap-2">
        <span
          className={cn(
            "inline-flex shrink-0 items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.16em]",
            toneClass,
          )}
        >
          {statusIcon}
          <span>{statusModel.badge}</span>
        </span>

        {/* One secondary line, never two: where the caret is beats how the
            document is doing, which the pill already said. */}
        {focusLabel ? (
          <span className="min-w-0 flex-1 truncate text-[11px] text-muted-foreground">
            <LocateFixed className="mr-1 inline h-3 w-3 align-[-2px]" />
            <span className="text-foreground/80">{focusLabel}</span>
          </span>
        ) : (
          <span className="min-w-0 flex-1 truncate text-[11px] text-muted-foreground">
            {statusModel.message}
          </span>
        )}

        {/* Only rendered when there is something to act on. A green "OK" badge
            next to a green "Ready" pill was two ways of saying "nothing is
            wrong", and silence says it better. */}
        {errorCount > 0 && onErrorsClick && (
          <button
            type="button"
            onClick={onErrorsClick}
            className="inline-flex shrink-0 items-center gap-1 rounded-full border border-destructive/40 bg-destructive/10 px-2 py-0.5 text-[10px] font-semibold text-destructive transition hover:bg-destructive/15"
          >
            <AlertCircle className="h-3 w-3" />
            <span>{errorCount}</span>
          </button>
        )}

        {shortcuts.length > 0 && (
          <button
            type="button"
            onClick={() => setExpanded((v) => !v)}
            aria-expanded={expanded}
            aria-label={expanded ? "Hide shortcuts" : "Show shortcuts"}
            className="inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-0.5 text-[10px] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            <Keyboard className="h-3 w-3" />
            {expanded
              ? <ChevronDown className="h-3 w-3" />
              : <ChevronUp className="h-3 w-3" />}
          </button>
        )}
      </div>

      {expanded && shortcuts.length > 0 && (
        <div className="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 border-t border-border/50 pt-1.5">
          {shortcuts.map((shortcut) => (
            <span
              key={shortcut.combo}
              className="inline-flex items-center gap-1.5 text-[10px] text-muted-foreground"
            >
              <kbd className="rounded border border-border/70 bg-muted/50 px-1.5 py-px font-mono text-[9px] font-semibold text-foreground/80">
                {shortcut.combo}
              </kbd>
              <span>{shortcut.label}</span>
            </span>
          ))}
        </div>
      )}
    </footer>
  );
}
