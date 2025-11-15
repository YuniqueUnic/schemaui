import { memo } from 'react';
import { clsx } from 'clsx';
import { Moon, Sun, Save, Power, Sparkles } from 'lucide-react';
import { useTheme } from '../theme';

interface AppHeaderProps {
  title?: string | null;
  description?: string | null;
  dirty: boolean;
  saving: boolean;
  onSave(): void;
  onSaveAndExit(): void;
  onExit(): void;
}

export const AppHeader = memo(function AppHeader({
  title,
  description,
  dirty,
  saving,
  onSave,
  onSaveAndExit,
  onExit,
}: AppHeaderProps) {
  const { theme, toggle } = useTheme();
  return (
    <header className="flex flex-wrap items-center justify-between gap-6 border-b border-slate-800/60 bg-slate-900/60 px-8 py-6 shadow-shell backdrop-blur-lg dark:bg-slate-950/70">
      <div className="min-w-0 flex-1">
        <p className="text-xs uppercase tracking-[0.3em] text-slate-400">
          SchemaUI Web
        </p>
        <h1 className="truncate text-2xl font-semibold text-white">
          {title || 'Configuration session'}
        </h1>
        {description ? (
          <p className="mt-1 line-clamp-2 text-sm text-slate-400">{description}</p>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <span
          className={clsx(
            'rounded-full border px-3 py-1 text-xs font-medium uppercase tracking-wide',
            dirty
              ? 'border-amber-400/60 text-amber-300'
              : 'border-emerald-400/50 text-emerald-200',
          )}
        >
          {dirty ? 'Unsaved' : 'Synced'}
        </span>
        <button
          type="button"
          onClick={toggle}
          className="inline-flex items-center gap-2 rounded-full border border-slate-700/70 px-3 py-2 text-sm font-medium text-slate-200 transition hover:border-slate-500 hover:text-white"
        >
          {theme === 'dark' ? (
            <>
              <Sun className="h-4 w-4" /> Light
            </>
          ) : (
            <>
              <Moon className="h-4 w-4" /> Dark
            </>
          )}
        </button>
        <button
          type="button"
          onClick={onSave}
          disabled={saving}
          className="inline-flex items-center gap-2 rounded-full border border-slate-600 px-4 py-2 text-sm font-semibold text-slate-100 transition hover:border-brand-400 hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Save className="h-4 w-4" />
          Save
        </button>
        <button
          type="button"
          onClick={onSaveAndExit}
          disabled={saving}
          className="inline-flex items-center gap-2 rounded-full border border-emerald-500/60 bg-emerald-500/10 px-4 py-2 text-sm font-semibold text-emerald-200 transition hover:bg-emerald-500/20 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Sparkles className="h-4 w-4" />
          Save & Exit
        </button>
        <button
          type="button"
          onClick={onExit}
          disabled={saving}
          className="inline-flex items-center gap-2 rounded-full border border-rose-500/50 text-rose-300 transition hover:bg-rose-500/10 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Power className="h-4 w-4" />
          Exit
        </button>
      </div>
    </header>
  );
});
