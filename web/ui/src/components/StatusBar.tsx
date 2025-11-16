interface ShortcutHint {
  combo: string;
  label: string;
}

interface StatusBarProps {
  status: string;
  dirty: boolean;
  validating: boolean;
  errorCount: number;
  shortcuts?: ShortcutHint[];
}

const DEFAULT_SHORTCUTS: ShortcutHint[] = [
  { combo: "⌘/Ctrl + S", label: "Save" },
  // { combo: "⌘/Ctrl + Enter", label: "Validate" },
  // { combo: "⌘/Ctrl + .", label: "Theme" },
];

export function StatusBar({
  status,
  dirty,
  validating,
  errorCount,
  shortcuts = DEFAULT_SHORTCUTS,
}: StatusBarProps) {
  return (
    <footer className="app-panel flex items-center justify-between border-t border-theme px-6 py-3 text-xs text-muted">
      <div className="flex items-center gap-3">
        <Badge
          label={dirty ? "Unsaved changes" : "Synced"}
          tone={dirty ? "amber" : "emerald"}
        />
        <Badge
          label={validating
            ? "Validating…"
            : errorCount > 0
            ? `${errorCount} errors`
            : "Schema valid"}
          tone={validating ? "sky" : errorCount > 0 ? "rose" : "emerald"}
        />
        <span className="text-[var(--app-text)]">{status}</span>
      </div>
      <div className="flex items-center gap-4">
        {shortcuts.map((shortcut) => (
          <span
            key={shortcut.combo}
            className="inline-flex items-center gap-2 text-[var(--app-text)]"
          >
            <kbd className="rounded-md border border-theme bg-[var(--app-panel-muted)] px-2 py-1 text-[10px] font-semibold text-[var(--app-text)]">
              {shortcut.combo}
            </kbd>
            <span className="text-muted">{shortcut.label}</span>
          </span>
        ))}
      </div>
    </footer>
  );
}

type BadgeTone = "emerald" | "rose" | "amber" | "sky";

function Badge({ label, tone }: { label: string; tone: BadgeTone }) {
  const styles: Record<BadgeTone, string> = {
    emerald: "border-emerald-600/60 text-emerald-400",
    rose: "border-rose-600/60 text-rose-400",
    amber: "border-amber-600/60 text-amber-400",
    sky: "border-sky-600/60 text-sky-400",
  };
  return (
    <span
      className={`rounded-full border px-3 py-1 text-[10px] font-semibold uppercase tracking-wide ${
        styles[tone]
      }`}
    >
      {label}
    </span>
  );
}
