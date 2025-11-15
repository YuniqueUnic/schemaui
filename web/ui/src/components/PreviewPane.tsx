import { memo } from 'react';
import { clsx } from 'clsx';
import { highlightSyntax } from '../utils/highlight';

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
  return (
    <aside className="flex h-full w-full flex-col border-l border-slate-800/70 bg-slate-950/70">
      <div className="flex items-center justify-between border-b border-slate-800/70 px-5 py-4">
        <div className="flex items-center gap-2">
          {formats.map((option) => (
            <button
              key={option}
              type="button"
              onClick={() => onFormatChange(option)}
              className={clsx(
                'rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-wide transition',
                option === format
                  ? 'bg-brand-500/20 text-brand-200'
                  : 'text-slate-400 hover:text-slate-200',
              )}
            >
              {option.toUpperCase()}
            </button>
          ))}
        </div>
        <label className="flex items-center gap-2 text-xs text-slate-400">
          <input
            type="checkbox"
            checked={pretty}
            onChange={(event) => onPrettyChange(event.target.checked)}
            className="h-4 w-4 accent-brand-400"
          />
          Pretty format
        </label>
      </div>
      <div className="relative flex-1 overflow-auto bg-slate-950 px-5 py-4 font-mono text-xs leading-relaxed text-slate-200">
        {loading ? (
          <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-slate-950/70">
            <div className="h-12 w-12 animate-spin rounded-full border-2 border-brand-400 border-t-transparent" />
          </div>
        ) : null}
        <pre className="whitespace-pre text-xs leading-relaxed">
          <code dangerouslySetInnerHTML={{ __html: highlightSyntax(payload, format) }} />
        </pre>
      </div>
    </aside>
  );
});
