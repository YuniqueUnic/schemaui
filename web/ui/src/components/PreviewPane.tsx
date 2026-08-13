import { memo, useCallback, useEffect, useRef, useState } from "react";
import { Check, ChevronRight, Copy, Eye } from "lucide-react";
import { highlightSyntax, normalizedLanguage } from "../utils/highlight";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Panel, PanelBody, PanelHeader } from "./Panel";
import { cn } from "@/lib/utils";

interface PreviewPaneProps {
  formats: string[];
  format: string;
  onFormatChange: (value: string) => void;
  pretty: boolean;
  onPrettyChange: (value: boolean) => void;
  payload: string;
  error?: string | null;
  loading?: boolean;
  onToggleCollapse?: () => void;
}

export const PreviewPane = memo(function PreviewPane({
  formats,
  format,
  onFormatChange,
  pretty,
  onPrettyChange,
  payload,
  error = null,
  loading = false,
  onToggleCollapse,
}: PreviewPaneProps) {
  const [copied, setCopied] = useState(false);
  const resetTimer = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (resetTimer.current) window.clearTimeout(resetTimer.current);
    };
  }, []);

  const handleCopy = useCallback(async () => {
    if (!payload) return;
    try {
      await navigator.clipboard.writeText(payload);
      setCopied(true);
      if (resetTimer.current) window.clearTimeout(resetTimer.current);
      resetTimer.current = window.setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  }, [payload]);

  return (
    <Panel as="aside" className="h-full w-full">
      <PanelHeader
        icon={<Eye className="h-3.5 w-3.5" />}
        label="Preview"
        actions={
          <div className="flex items-center gap-1">
            <Button
              variant="outline"
              size="sm"
              onClick={handleCopy}
              disabled={!payload || !!error}
              className="h-7 rounded-full text-xs"
            >
              {copied
                ? <Check className="h-3.5 w-3.5 text-emerald-500" />
                : <Copy className="h-3.5 w-3.5" />}
              <span className="ml-1">{copied ? "Copied" : "Copy"}</span>
            </Button>
            {onToggleCollapse && (
              <button
                type="button"
                onClick={onToggleCollapse}
                aria-label="Collapse preview"
                className="flex items-center justify-center rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
              >
                <ChevronRight className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        }
      />
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-theme px-4 py-2.5">
        <div className="inline-flex items-center gap-1 rounded-full bg-muted/60 p-0.5">
          {formats.map((option) => {
            const active = option === format;
            return (
              <button
                key={option}
                type="button"
                onClick={() => onFormatChange(option)}
                className={cn(
                  "rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.14em] transition-colors",
                  active
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {option}
              </button>
            );
          })}
        </div>
        <div className="flex items-center gap-2">
          <Switch
            id="pretty-format"
            checked={pretty}
            onCheckedChange={onPrettyChange}
          />
          <Label
            htmlFor="pretty-format"
            className="cursor-pointer text-xs text-muted-foreground"
          >
            Pretty
          </Label>
        </div>
      </div>
      <PanelBody className="relative bg-[color-mix(in_oklch,var(--color-panel-muted)_60%,transparent)] px-4 py-4">
        {loading && (
          <div className="pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-background/60 backdrop-blur-sm">
            <div className="h-10 w-10 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          </div>
        )}
        {error
          ? (
            <div className="relative h-full overflow-auto whitespace-pre-wrap break-words rounded-xl border border-destructive/30 bg-destructive/5 px-4 py-3 text-xs leading-relaxed shadow-inner">
              <p className="font-medium text-destructive">Preview failed</p>
              <p className="mt-1 font-mono text-foreground">{error}</p>
            </div>
          )
          : (
            <pre className="relative h-full overflow-auto whitespace-pre-wrap break-words rounded-xl border border-theme bg-card px-4 py-3 font-mono text-xs leading-relaxed shadow-inner">
              <code
                className={`language-${normalizedLanguage(format)}`}
                dangerouslySetInnerHTML={{
                  __html: highlightSyntax(payload, format),
                }}
              />
            </pre>
          )}
      </PanelBody>
    </Panel>
  );
});
