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
  { combo: "⌘/Ctrl + S", label: "Save" }, // ONLY THIS COMMAND IS USEFUL. DO NOT DELETE
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
    <footer className="flex items-center justify-between border-t border-slate-800/70 bg-slate-950/70 px-6 py-3 text-xs text-slate-300">
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
        <span>{status}</span>
      </div>
      <div className="flex items-center gap-4">
        {shortcuts.map((shortcut) => (
          <span
            key={shortcut.combo}
            className="inline-flex items-center gap-2 text-slate-400"
          >
            <kbd className="rounded-md border border-slate-800 bg-slate-900/60 px-2 py-1 text-[10px] font-semibold text-slate-100">
              {shortcut.combo}
            </kbd>
            <span>{shortcut.label}</span>
          </span>
        ))}
      </div>
    </footer>
  );
}

type BadgeTone = "emerald" | "rose" | "amber" | "sky";

function Badge({ label, tone }: { label: string; tone: BadgeTone }) {
  const styles: Record<BadgeTone, string> = {
    emerald: "border-emerald-400/60 text-emerald-200",
    rose: "border-rose-400/60 text-rose-200",
    amber: "border-amber-400/60 text-amber-200",
    sky: "border-sky-400/60 text-sky-200",
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
