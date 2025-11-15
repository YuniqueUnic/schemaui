import { memo } from 'react';
import { clsx } from 'clsx';

interface ShortcutHint {
  combo: string;
  label: string;
}

interface StatusBarProps {
  status: string;
  dirty: boolean;
  validating: boolean;
  errorCount: number;
  lastSaved?: Date | null;
  shortcuts?: ShortcutHint[];
}

export const StatusBar = memo(function StatusBar({
  status,
  dirty,
  validating,
  errorCount,
  lastSaved,
  shortcuts = [],
}: StatusBarProps) {
  return (
    <footer className="flex flex-wrap items-center justify-between gap-4 border-t border-slate-200 bg-white/90 px-6 py-3 text-xs text-slate-600 dark:border-slate-800/70 dark:bg-slate-950/80 dark:text-slate-400">
      <div className="flex items-center gap-3">
        <Badge
          label={dirty ? 'Pending changes' : 'All changes saved'}
          tone={dirty ? 'amber' : 'emerald'}
        />
        <Badge
          label={
            validating
              ? 'Validating…'
              : errorCount > 0
                ? `${errorCount} error(s)`
                : 'Schema valid'
          }
          tone={
            validating ? 'sky' : errorCount > 0 ? 'rose' : ('emerald' as BadgeTone)
          }
        />
        {lastSaved ? (
          <span>Last saved {lastSaved.toLocaleTimeString()}</span>
        ) : null}
      </div>
      <div className="flex flex-wrap items-center gap-4">
        {shortcuts.map((shortcut) => (
          <span key={shortcut.combo} className="inline-flex items-center gap-2">
            <kbd className="rounded border border-slate-200 bg-white px-2 py-1 font-semibold text-slate-700 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200">
              {shortcut.combo}
            </kbd>
            <span>{shortcut.label}</span>
          </span>
        ))}
        <p>{status}</p>
      </div>
    </footer>
  );
});

type BadgeTone = 'emerald' | 'rose' | 'amber' | 'sky';

function Badge({ label, tone }: { label: string; tone: BadgeTone }) {
  const styles: Record<BadgeTone, string> = {
    emerald:
      'border-emerald-400/70 text-emerald-700 dark:border-emerald-400/50 dark:text-emerald-200',
    rose: 'border-rose-400/70 text-rose-600 dark:border-rose-400/50 dark:text-rose-200',
    amber: 'border-amber-400/70 text-amber-700 dark:border-amber-400/50 dark:text-amber-200',
    sky: 'border-sky-400/70 text-sky-700 dark:border-sky-400/50 dark:text-sky-200',
  };
  return (
    <span
      className={clsx(
        'rounded-full border px-3 py-1 text-[10px] font-semibold uppercase',
        styles[tone],
      )}
    >
      {label}
    </span>
  );
}
