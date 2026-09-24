import { useEffect, useMemo, useRef, useState } from "react";
import { Textarea } from "../ui/textarea";
import { DiagramStage, type FigureDescriptor } from "./RichSurface";
import { useEditorRuntime } from "./EditorRuntime";
import { useTheme } from "../../theme";
import { useI18n } from "../../i18n";
import type { RenderedSvg } from "../../types";

/** How the editor and the preview share the row. */
type EditorLayout = { mode: "split" | "stacked"; swapped: boolean };

const LAYOUT_KEY = "schemaui-mermaid-layout";
const FOLLOW_DELAY_MS = 300;

function loadLayout(): EditorLayout {
  try {
    const raw = localStorage.getItem(LAYOUT_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as EditorLayout;
      if (parsed.mode === "split" || parsed.mode === "stacked") {
        return { mode: parsed.mode, swapped: parsed.swapped === true };
      }
    }
  } catch {
    // localStorage may be unavailable (tests, privacy modes); defaults are fine.
  }
  return { mode: "split", swapped: false };
}

/**
 * A Mermaid source editor with a live rendered preview.
 *
 * The value on the wire is the source itself — this control only adds a
 * window onto it. Two properties keep the window from fighting the person
 * typing:
 *
 * - **The pane never collapses or reshuffles.** Both halves have a fixed
 *   height, and while a render is in flight the *previous* diagram stays on
 *   screen under a small "rendering" pill. The old behaviour — blank pane,
 *   layout jumping as the new diagram streamed in — made every keystroke
 *   feel like a page reload.
 * - **The arrangement is the author's choice**: side by side, stacked, or
 *   side by side the other way round; persisted in localStorage. Preview
 *   updates can also be switched from live-follow to explicit render — the
 *   edit-then-commit reading — for sources big enough that watching them
 *   redraw per keystroke is noise.
 *
 * The preview is a first-class figure: it goes through `DiagramStage` with a
 * descriptor, so hovering enlarges it and clicking opens the fullscreen
 * viewer (copy/download included) exactly like every declared figure.
 *
 * Without a renderer — an older engine, a wasm package built without the
 * feature — this is a plain multi-line text area, which is exactly what the
 * field was before the control existed.
 */
export function MermaidEditorControl({
  value,
  onChange,
}: {
  value: string;
  onChange: (next: string) => void;
}) {
  const { t } = useI18n();
  const { renderContent } = useEditorRuntime();
  const { theme } = useTheme();
  const [layout, setLayout] = useState<EditorLayout>(loadLayout);
  const [follow, setFollow] = useState(true);
  const [lastGood, setLastGood] = useState<RenderedSvg | null>(null);
  const [rendering, setRendering] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const seqRef = useRef(0);
  const cacheRef = useRef(new Map<string, RenderedSvg>());
  // The source the current `lastGood` was rendered from: a manual-mode
  // button compares against this to know the pane is stale, and the
  // viewer's copy/download must show the source that is actually on screen.
  const renderedSourceRef = useRef<string | null>(null);

  const cacheKey = `${theme}\0${value}`;
  const cached = useMemo(() => cacheRef.current.get(cacheKey), [cacheKey]);
  const stale = follow ? false : renderedSourceRef.current !== value;

  const render = () => {
    if (!renderContent) return;
    const seq = ++seqRef.current;
    setRendering(true);
    void renderContent({ kind: "mermaid", source: value, theme }).then((result) => {
      // Only the newest request may speak; an earlier one that lands late is
      // describing a source the user has already left behind.
      if (seq !== seqRef.current) return;
      setRendering(false);
      if ("svg" in result) {
        cacheRef.current.set(cacheKey, result.svg);
        renderedSourceRef.current = value;
        setLastGood(result.svg);
        setError(null);
      } else {
        // A parse error keeps the last good diagram on screen — the message
        // rides along under it — instead of blanking the pane.
        setError(result.error.message);
      }
    });
  };

  useEffect(() => {
    if (!renderContent) return;
    if (value.trim() === "") {
      seqRef.current++;
      setRendering(false);
      setError(null);
      setLastGood(null);
      renderedSourceRef.current = null;
      return;
    }
    if (cached) {
      renderedSourceRef.current = value;
      setLastGood(cached);
      setError(null);
      setRendering(false);
      return;
    }
    if (!follow) return;
    const timer = window.setTimeout(render, FOLLOW_DELAY_MS);
    return () => window.clearTimeout(timer);
    // `render` closes over the current value/theme; the timer is the only
    // path that needs re-arming.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [renderContent, value, theme, cacheKey, cached, follow]);

  const updateLayout = (next: Partial<EditorLayout>) => {
    setLayout((current) => {
      const merged = { ...current, ...next };
      try {
        localStorage.setItem(LAYOUT_KEY, JSON.stringify(merged));
      } catch {
        // Non-durable layout is a preference, not a promise.
      }
      return merged;
    });
  };

  if (!renderContent) {
    return (
      <Textarea
        value={value}
        onChange={(event) => onChange(event.target.value)}
        spellCheck={false}
        aria-label={t("Mermaid source")}
        className="min-h-40 font-mono text-xs"
      />
    );
  }

  const stacked = layout.mode === "stacked";
  const editorPane = (
    <Textarea
      value={value}
      onChange={(event) => onChange(event.target.value)}
      spellCheck={false}
      aria-label={t("Mermaid source")}
      className="h-56 resize-none font-mono text-xs sm:h-64"
    />
  );
  const previewSource = renderedSourceRef.current ?? value;
  const previewFigure: FigureDescriptor | undefined = lastGood
    ? {
      svg: lastGood,
      title: t("Mermaid preview"),
      content: { type: "mermaid", source: previewSource },
    }
    : undefined;
  const previewPane = (
    <div className="relative h-56 sm:h-64" data-testid="mermaid-preview">
      {lastGood
        ? (
          <DiagramStage
            svg={lastGood}
            label={t("Mermaid preview")}
            fit="fill"
            className="h-full w-full"
            figure={previewFigure}
          />
        )
        : error
        ? <ErrorCard message={error} />
        : (
          <div className="flex h-full items-center justify-center rounded-md border border-dashed border-border text-xs text-muted-foreground">
            {t("Type Mermaid source to see it rendered")}
          </div>
        )}
      {rendering && (
        <span className="absolute right-2 top-2 rounded-full border border-border bg-background/90 px-2 py-0.5 text-[10px] text-muted-foreground">
          {t("Rendering…")}
        </span>
      )}
      {error && lastGood && (
        <p
          title={error}
          className="absolute inset-x-2 bottom-2 truncate rounded-md border border-destructive/30 bg-destructive/10 px-2 py-1 text-[11px] text-destructive"
        >
          {error}
        </p>
      )}
    </div>
  );

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground">
        <ToolButton
          label={t("Editor left, preview right")}
          pressed={!stacked && !layout.swapped}
          onClick={() => updateLayout({ mode: "split", swapped: false })}
        >
          {t("Side by side")}
        </ToolButton>
        <ToolButton
          label={t("Editor above, preview below")}
          pressed={stacked}
          onClick={() => updateLayout({ mode: "stacked", swapped: false })}
        >
          {t("Stacked")}
        </ToolButton>
        <ToolButton
          label={t("Preview left, editor right")}
          pressed={!stacked && layout.swapped}
          disabled={stacked}
          onClick={() => updateLayout({ swapped: true })}
        >
          {t("Swap sides")}
        </ToolButton>
        <span className="mx-1 h-4 w-px bg-border" />
        <ToolButton
          label={t("Preview follows typing")}
          pressed={follow}
          onClick={() => setFollow((current) => !current)}
        >
          {t("Live")}
        </ToolButton>
        {!follow && stale && (
          <button
            type="button"
            onClick={render}
            className="rounded-md border border-border px-2 py-0.5 hover:bg-muted"
          >
            {t("Render preview")}
          </button>
        )}
      </div>
      <div className={`grid gap-3 ${stacked ? "" : "lg:grid-cols-2"}`}>
        <div className={stacked ? "" : layout.swapped ? "order-2" : "order-1"}>
          {editorPane}
        </div>
        <div className={stacked ? "" : layout.swapped ? "order-1" : "order-2"}>
          {previewPane}
        </div>
      </div>
    </div>
  );
}

function ToolButton({
  label,
  pressed,
  disabled,
  onClick,
  children,
}: {
  label: string;
  pressed: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      aria-pressed={pressed}
      disabled={disabled}
      onClick={onClick}
      className={`rounded-md border px-2 py-0.5 transition-colors ${
        pressed
          ? "border-foreground/30 bg-foreground/10 text-foreground"
          : "border-border hover:bg-muted"
      } disabled:cursor-not-allowed disabled:opacity-40`}
    >
      {children}
    </button>
  );
}

function ErrorCard({ message }: { message: string }) {
  const { t } = useI18n();
  return (
    <div className="flex h-full flex-col justify-center gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3">
      <p className="text-xs font-medium text-destructive">{t("This diagram does not parse:")}</p>
      <p className="text-xs text-destructive/90">{message}</p>
    </div>
  );
}
