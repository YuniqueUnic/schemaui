import { memo, useCallback, useEffect, useRef, useState } from "react";
import { clsx } from "clsx";
import { highlightSyntax } from "../utils/highlight";

interface PreviewPaneProps {
  formats: string[];
  format: string;
  onFormatChange: (value: string) => void;
  pretty: boolean;
  onPrettyChange: (value: boolean) => void;
  payload: string;
  loading?: boolean;
}

export const PreviewPane = memo(function PreviewPane({
  formats,
  format,
  onFormatChange,
  pretty,
  onPrettyChange,
  payload,
  loading = false,
}: PreviewPaneProps) {
  const [copied, setCopied] = useState(false);
  const resetTimer = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (resetTimer.current) {
        window.clearTimeout(resetTimer.current);
      }
    };
  }, []);

  const handleCopy = useCallback(async () => {
    if (!payload) {
      return;
    }
    try {
      await navigator.clipboard.writeText(payload);
      setCopied(true);
      if (resetTimer.current) {
        window.clearTimeout(resetTimer.current);
      }
      resetTimer.current = window.setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  }, [payload]);

  return (
    <aside className="flex h-full w-full flex-col border-l border-slate-200 bg-white dark:border-slate-800/70 dark:bg-slate-950/70">
      <div className="flex items-center justify-between border-b border-slate-200 px-5 py-4 dark:border-slate-800/70">
        <div className="flex items-center gap-2">
          {formats.map((option) => (
            <button
              key={option}
              type="button"
              onClick={() => onFormatChange(option)}
              className={clsx(
                "rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-wide transition",
                option === format
                  ? "bg-brand-500/20 text-brand-700 dark:text-brand-200"
                  : "text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200",
              )}
            >
              {option.toUpperCase()}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
            <input
              type="checkbox"
              checked={pretty}
              onChange={(event) => onPrettyChange(event.target.checked)}
              className="h-4 w-4 accent-brand-400"
            />
            Pretty format
          </label>
          <button
            type="button"
            onClick={handleCopy}
            disabled={!payload}
            className="rounded-full border border-slate-300 px-3 py-1.5 text-xs font-semibold text-slate-700 transition hover:border-brand-400 hover:text-brand-600 disabled:cursor-not-allowed disabled:opacity-50 dark:border-slate-600 dark:text-slate-200"
          >
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
      <div className="relative flex-1 overflow-auto bg-white px-5 py-4 font-mono text-xs leading-relaxed text-slate-800 dark:bg-slate-950 dark:text-slate-200">
        {loading
          ? (
            <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-white/70 dark:bg-slate-950/70">
              <div className="h-12 w-12 animate-spin rounded-full border-2 border-brand-400 border-t-transparent" />
            </div>
          )
          : null}
        <pre className="whitespace-pre text-xs leading-relaxed">
          <code dangerouslySetInnerHTML={{ __html: highlightSyntax(payload, format) }} />
        </pre>
      </div>
    </aside>
  );
});
