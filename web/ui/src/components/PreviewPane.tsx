import { memo, useCallback, useEffect, useRef, useState } from "react";
import { highlightSyntax, normalizedLanguage } from "../utils/highlight";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";

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
            <Button
              key={option}
              variant={option === format ? "default" : "ghost"}
              size="sm"
              onClick={() => onFormatChange(option)}
              className="rounded-full text-xs font-semibold uppercase tracking-wide"
            >
              {option.toUpperCase()}
            </Button>
          ))}
        </div>
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <Switch
              id="pretty-format"
              checked={pretty}
              onCheckedChange={onPrettyChange}
            />
            <Label
              htmlFor="pretty-format"
              className="text-xs text-muted-foreground cursor-pointer"
            >
              Pretty format
            </Label>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleCopy}
            disabled={!payload}
          >
            {copied ? "Copied" : "Copy"}
          </Button>
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
