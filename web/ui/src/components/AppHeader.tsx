import { memo } from "react";
import { clsx } from "clsx";
import { Moon, Power, Save, Sun } from "lucide-react";
import { useTheme } from "../theme";

interface AppHeaderProps {
  title?: string | null;
  description?: string | null;
  dirty: boolean;
  saving: boolean;
  exiting?: boolean;
  onSave(): void;
  onExit(): void;
}

export const AppHeader = memo(function AppHeader({
  title,
  description,
  dirty,
  saving,
  exiting = false,
  onSave,
  onExit,
}: AppHeaderProps) {
  const { theme, toggle } = useTheme();
  return (
    <header className="app-panel flex flex-wrap items-center justify-between gap-6 border-b border-theme px-8 py-6 text-[var(--app-text)]">
      <div className="min-w-0 flex-1">
        <p className="text-xs uppercase tracking-[0.3em] text-muted">
          SchemaUI Web
        </p>
        <h1 className="truncate text-lg font-semibold">
          {title || "Configuration session"}
        </h1>
        {description
          ? (
            <p className="mt-1 line-clamp-2 text-sm text-muted">
              {description}
            </p>
          )
          : null}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <span
          className={clsx(
            "rounded-full border px-4 py-1 text-sm font-medium uppercase tracking-wide",
            dirty
              ? "border-amber-600/60 text-amber-400"
              : "border-emerald-600/60 text-emerald-400",
          )}
        >
          {dirty ? "Unsaved" : "Synced"}
        </span>

        <button
          type="button"
          onClick={onSave}
          disabled={saving}
          className="inline-flex items-center gap-2 rounded-full border border-theme px-4 py-1 text-sm font-semibold text-[var(--app-text)] transition hover:text-[var(--app-accent)] disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Save className="h-4 w-4" />
          Save
        </button>
        <button
          type="button"
          onClick={toggle}
          className="inline-flex items-center gap-2 rounded-full border border-theme px-3 py-1 text-sm font-medium text-[var(--app-text)] transition hover:text-[var(--app-accent)]"
        >
          {theme === "dark"
            ? (
              <>
                <Sun className="h-4 w-4" /> Light
              </>
            )
            : (
              <>
                <Moon className="h-4 w-4" /> Dark
              </>
            )}
        </button>
        <button
          type="button"
          onClick={onExit}
          disabled={saving || exiting}
          className="inline-flex items-center gap-2 rounded-full border border-rose-400/80 px-4 py-1 text-sm text-rose-400 transition hover:bg-rose-400/40 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Power className="h-4 w-4" />
          {exiting ? "Exiting…" : "Exit"}
        </button>
      </div>
    </header>
  );
});
