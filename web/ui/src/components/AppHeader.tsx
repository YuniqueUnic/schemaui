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
    <header className="flex flex-wrap items-center justify-between gap-6 border-b border-slate-100 bg-white/85 px-8 py-6 text-slate-900 backdrop-blur-lg dark:border-slate-800/60 dark:bg-slate-950/70 dark:text-white">
      <div className="min-w-0 flex-1">
        <p className="text-xs uppercase tracking-[0.3em] text-slate-500 dark:text-slate-400">
          SchemaUI Web
        </p>
        <h1 className="truncate text-lg font-semibold">
          {title || "Configuration session"}
        </h1>
        {description
          ? (
            <p className="mt-1 line-clamp-2 text-sm text-slate-600 dark:text-slate-400">
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
              ? "border-amber-400/80 text-amber-600 dark:border-amber-400/60 dark:text-amber-300"
              : "border-emerald-400/70 text-emerald-700 dark:border-emerald-400/50 dark:text-emerald-200",
          )}
        >
          {dirty ? "Unsaved" : "Synced"}
        </span>

        <button
          type="button"
          onClick={onSave}
          disabled={saving}
          className="inline-flex items-center gap-2 rounded-full border border-slate-300 bg-white px-4 py-1 text-sm font-semibold text-slate-800 transition hover:border-brand-400 hover:text-brand-600 disabled:cursor-not-allowed disabled:opacity-50 dark:border-slate-600 dark:bg-transparent dark:text-slate-100 dark:hover:text-white"
        >
          <Save className="h-4 w-4" />
          Save
        </button>
        <button
          type="button"
          onClick={toggle}
          className="inline-flex items-center gap-2 rounded-full border border-slate-300 px-3 py-1 text-sm font-medium text-slate-700 transition hover:border-slate-500 hover:text-slate-900 dark:border-slate-700/70 dark:text-slate-200 dark:hover:text-white"
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
          className="inline-flex items-center gap-2 px-4 py-1 text-sm rounded-full border border-rose-400 text-rose-600 transition hover:bg-rose-500/10 dark:border-rose-500/50 dark:text-rose-300 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Power className="h-4 w-4" />
          {exiting ? "Exiting…" : "Exit"}
        </button>
      </div>
    </header>
  );
});
