import { memo } from "react";
import { AlertTriangle } from "lucide-react";

interface ExitGuardDialogProps {
  errors: Array<[string, string]>;
  forcing: boolean;
  onCancel(): void;
  onForceExit(): void;
}

export const ExitGuardDialog = memo(function ExitGuardDialog({
  errors,
  forcing,
  onCancel,
  onForceExit,
}: ExitGuardDialogProps) {
  const preview = errors.slice(0, 6);
  const remaining = Math.max(0, errors.length - preview.length);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/70 px-4 py-6">
      <div
        role="dialog"
        aria-modal="true"
        className="w-full max-w-2xl rounded-3xl border border-rose-400/40 bg-white/95 p-6 shadow-2xl backdrop-blur-sm dark:border-rose-500/60 dark:bg-slate-950/90"
      >
        <div className="flex items-center gap-3">
          <span className="rounded-full bg-rose-100/70 p-2 text-rose-600 dark:bg-rose-500/20 dark:text-rose-200">
            <AlertTriangle className="h-6 w-6" />
          </span>
          <div>
            <p className="text-sm uppercase tracking-[0.25em] text-rose-500 dark:text-rose-200">
              Exit blocked
            </p>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white">
              Resolve the remaining schema errors
            </h3>
            <p className="text-sm text-slate-600 dark:text-slate-300">
              Fix the issues below or force exit to emit only the last saved
              configuration.
            </p>
          </div>
        </div>
        <ul className="mt-4 space-y-3 text-sm text-slate-700 dark:text-slate-200">
          {preview.map(([pointer, message]) => (
            <li
              key={pointer + message}
              className="rounded-2xl border border-rose-200 bg-rose-50/60 px-4 py-2 dark:border-rose-500/40 dark:bg-rose-500/10"
            >
              <p className="font-mono text-xs text-rose-500">
                {pointer || "/"}
              </p>
              <p>{message}</p>
            </li>
          ))}
        </ul>
        {remaining > 0
          ? (
            <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">
              +{remaining}{" "}
              more issue(s) hidden. Continue editing to see full details.
            </p>
          )
          : null}
        <div className="mt-6 flex flex-wrap justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            className="rounded-full border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 transition hover:border-slate-500 hover:text-slate-900 dark:border-slate-600 dark:text-slate-200 dark:hover:text-white"
          >
            Back to form
          </button>
          <button
            type="button"
            onClick={onForceExit}
            disabled={forcing}
            className="inline-flex items-center gap-2 rounded-full border border-rose-500 bg-rose-500/10 px-4 py-2 text-sm font-semibold text-rose-600 transition hover:bg-rose-500/20 disabled:cursor-not-allowed disabled:opacity-50 dark:border-rose-400 dark:text-rose-200"
          >
            {forcing ? "Exiting…" : "Force exit with last saved data"}
          </button>
        </div>
      </div>
    </div>
  );
});
