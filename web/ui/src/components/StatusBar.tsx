import { Badge } from "@/components/ui/badge";

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
          variant={dirty ? "default" : "secondary"}
          className="text-[10px] uppercase tracking-wide"
        >
          {dirty ? "Unsaved changes" : "Synced"}
        </Badge>
        <Badge
          variant={errorCount > 0 ? "destructive" : "secondary"}
          className="text-[10px] uppercase tracking-wide"
        >
          {validating
            ? "Validating…"
            : errorCount > 0
            ? `${errorCount} errors`
            : "Schema valid"}
        </Badge>
        <span className="text-[var(--app-text)]">{status}</span>
      </div>
      <div className="flex items-center gap-4">
        {shortcuts.map((shortcut) => (
          <span
            key={shortcut.combo}
            className="inline-flex items-center gap-2 text-[var(--app-text)]"
          >
            <kbd className="rounded-md border border-input bg-muted px-2 py-1 text-[10px] font-semibold">
              {shortcut.combo}
            </kbd>
            <span className="text-muted-foreground">{shortcut.label}</span>
          </span>
        ))}
      </div>
    </footer>
  );
}
