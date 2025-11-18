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
  onErrorsClick?: () => void;
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
  onErrorsClick,
}: StatusBarProps) {
  return (
    <footer className="app-panel flex flex-wrap items-center justify-between gap-2 md:gap-4 border-t border-theme px-4 md:px-6 py-2 md:py-3 text-xs text-muted">
      <div className="flex items-center gap-2 md:gap-3 flex-wrap">
        <Badge
          variant={dirty ? "default" : "secondary"}
          className="text-[10px] uppercase tracking-wide"
        >
          {dirty ? "Unsaved" : "Synced"}
        </Badge>
        <Badge
          variant={errorCount > 0 ? "destructive" : "secondary"}
          className={`text-[10px] uppercase tracking-wide ${
            errorCount > 0 && onErrorsClick
              ? "cursor-pointer hover:opacity-80"
              : ""
          }`}
          onClick={errorCount > 0 && onErrorsClick ? onErrorsClick : undefined}
        >
          {validating
            ? "Validating…"
            : errorCount > 0
            ? `${errorCount} error${errorCount > 1 ? "s" : ""}`
            : "Valid"}
        </Badge>
        <span className="text-[var(--app-text)] hidden md:inline">
          {status}
        </span>
      </div>
      <div className="hidden lg:flex items-center gap-4">
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
