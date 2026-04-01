import {
  AlertCircle,
  CheckCircle2,
  Keyboard,
  LoaderCircle,
  LocateFixed,
  Save,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
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
  const statusModel = buildStatusModel({
    status,
    dirty,
    validating,
    saving,
    exiting,
    errorCount,
  });

  const toneClass = {
    ready:
      "border-emerald-500/30 bg-emerald-500/12 text-emerald-700 dark:text-emerald-300",
    dirty:
      "border-amber-500/30 bg-amber-500/12 text-amber-700 dark:text-amber-300",
    error:
      "border-rose-500/30 bg-rose-500/12 text-rose-700 dark:text-rose-300",
    busy:
      "border-sky-500/30 bg-sky-500/12 text-sky-700 dark:text-sky-300",
  }[statusModel.tone];

  const statusIcon = statusModel.tone === "error"
    ? <AlertCircle className="h-3.5 w-3.5" />
    : statusModel.tone === "busy"
    ? <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
    : statusModel.tone === "dirty"
    ? <Save className="h-3.5 w-3.5" />
    : <CheckCircle2 className="h-3.5 w-3.5" />;

  return (
    <footer className="border-t border-border/60 bg-background/95 px-4 py-3 text-xs backdrop-blur md:px-6">
      <div className="grid gap-2 xl:grid-cols-[minmax(0,1.15fr)_minmax(20rem,0.85fr)]">
        <section className="rounded-2xl border border-border/60 bg-muted/25 px-3 py-2.5">
          <div className="flex items-center gap-2 text-[10px] uppercase tracking-[0.24em] text-muted-foreground">
            <Keyboard className="h-3.5 w-3.5" />
            <span>Shortcuts</span>
            <span className="rounded-full border border-border/60 bg-background/80 px-2 py-0.5 text-[9px] tracking-[0.18em] text-foreground/70">
              Context aware
            </span>
          </div>
          <div className="mt-2 flex gap-2 overflow-x-auto pb-1">
            {shortcuts.map((shortcut) => (
              <span
                key={shortcut.combo}
                className="inline-flex shrink-0 items-center gap-2 rounded-xl border border-border/70 bg-background/90 px-2.5 py-1 shadow-sm"
              >
                <kbd className="rounded-md bg-primary/10 px-2 py-0.5 font-mono text-[10px] font-semibold text-primary">
                  {shortcut.combo}
                </kbd>
                <span className="text-[11px] font-medium text-foreground/90">
                  {shortcut.label}
                </span>
              </span>
            ))}
          </div>
        </section>

        <section className="rounded-2xl border border-border/60 bg-card/70 px-3 py-2.5 shadow-sm">
          <div className="flex flex-wrap items-center gap-2">
            <span
              className={cn(
                "inline-flex items-center gap-1 rounded-full border px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.22em]",
                toneClass,
              )}
            >
              {statusIcon}
              <span>{statusModel.badge}</span>
            </span>

            {focusLabel && (
              <span className="inline-flex max-w-full items-center gap-1 rounded-full border border-border/70 bg-background/80 px-2.5 py-1 text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
                <LocateFixed className="h-3.5 w-3.5 shrink-0" />
                <span className="truncate">
                  Focus
                  <span className="ml-1 normal-case tracking-normal text-foreground">
                    {focusLabel}
                  </span>
                </span>
              </span>
            )}

            {errorCount > 0 && onErrorsClick
              ? (
                <button
                  type="button"
                  onClick={onErrorsClick}
                  className="inline-flex items-center gap-1 rounded-full border border-rose-500/30 bg-rose-500/10 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.18em] text-rose-700 transition hover:bg-rose-500/15 dark:text-rose-300"
                >
                  <AlertCircle className="h-3.5 w-3.5" />
                  <span>Open errors</span>
                </button>
              )
              : (
                <Badge
                  variant="secondary"
                  className="border border-emerald-500/20 bg-emerald-500/10 text-[10px] uppercase tracking-[0.18em] text-emerald-700 dark:text-emerald-300"
                >
                  Validated
                </Badge>
              )}
          </div>

          <p className="mt-2 text-[12px] leading-5 text-foreground/90">
            {statusModel.message}
          </p>
        </section>
      </div>
    </footer>
  );
}
