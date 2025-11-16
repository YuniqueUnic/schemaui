import { memo, useCallback, useEffect, useRef, useState } from "react";
import { clsx } from "clsx";
import { highlightSyntax, normalizedLanguage } from "../utils/highlight";

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
    <aside className="flex h-full w-full flex-col app-panel">
      <div className="flex items-center justify-between border-b border-theme px-5 py-4">
        <div className="flex items-center gap-2">
          {formats.map((option) => (
            <button
              key={option}
              type="button"
              onClick={() => onFormatChange(option)}
              className={clsx(
                "rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-wide transition",
                option === format
                  ? "bg-[var(--app-accent)]/20 text-[var(--app-accent)]"
                  : "text-muted hover:text-[var(--app-accent)]",
              )}
            >
              {option.toUpperCase()}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 text-xs text-muted">
            <input
              type="checkbox"
              checked={pretty}
              onChange={(event) => onPrettyChange(event.target.checked)}
              className="h-4 w-4 accent-[var(--app-accent)]"
            />
            Pretty format
          </label>
          <button
            type="button"
            onClick={handleCopy}
            disabled={!payload}
            className="rounded-full border border-theme px-3 py-1.5 text-xs font-semibold text-[var(--app-text)] transition hover:text-[var(--app-accent)] disabled:cursor-not-allowed disabled:opacity-50"
          >
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
      </div>
      <div className="relative flex-1 overflow-auto bg-[var(--app-panel)] px-5 py-4 font-mono text-xs leading-relaxed text-[var(--app-text)]">
        {loading
          ? (
            <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-[var(--app-panel)]/80">
              <div className="h-12 w-12 animate-spin rounded-full border-2 border-[var(--app-accent)] border-t-transparent" />
            </div>
          )
          : null}
        <pre className="relative whitespace-pre-wrap break-words rounded-xl bg-[var(--app-card)] px-4 py-3 text-xs leading-relaxed text-[var(--app-text)]">
          <code
            className={`language-${normalizedLanguage(format)}`}
            dangerouslySetInnerHTML={{ __html: highlightSyntax(payload, format) }}
          />
        </pre>
      </div>
    </aside>
  );
});
