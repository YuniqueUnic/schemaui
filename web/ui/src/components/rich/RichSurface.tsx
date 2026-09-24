import { useOverlay } from "../Overlay";
import { useTheme } from "../../theme";
import { useI18n } from "../../i18n";
import { useRichAssets } from "./RichAssetsProvider";
import { useHoverPreview } from "./HoverPreviewLayer";
import type { RenderedSvg, RichContent } from "../../types";

/**
 * How much room a figure gets, and how it is allowed to use it:
 *
 * - `mini` — inside a select option; shrinks to the row, hover-only;
 * - `thumb` — an option's figure beside or under its control;
 * - `block` — a figure declared on the node itself;
 * - `viewer` — the fullscreen viewer stage, where the figure fills the room
 *   it is given (up *and* down) instead of sitting at its intrinsic size.
 *
 * The hover layer's floating enlargement lives in HoverPreviewLayer, which
 * stages a `DiagramStage` directly.
 */
export type RichVariant = "mini" | "thumb" | "block" | "viewer";

/**
 * What hover/click need to know about a figure, wherever it lives — a static
 * asset resolved from the session map or a diagram rendered on the fly (the
 * Mermaid editor's preview). `content` is the declared source when there is
 * one; it powers the viewer's copy/download. This descriptor is the whole
 * global interaction contract: anything that can hand one to `DiagramStage`
 * gets the same hover-to-enlarge and click-to-view behaviour.
 */
export interface FigureDescriptor {
  svg: RenderedSvg;
  /** Names the figure in the fullscreen viewer and its aria-label. */
  title?: string;
  content?: RichContent;
}

/** Rendered SVG fitted into a box the caller sizes. */
export function DiagramStage({
  svg,
  label,
  fit = "contain",
  className,
  bare = false,
  figure,
  interaction = "full",
}: {
  svg: RenderedSvg;
  label: string;
  /** `fill` lets the drawing scale up to the box; `contain` only shrinks. */
  fit?: "fill" | "contain";
  className?: string;
  /** `true` drops the framed look for figures embedded in tight rows. */
  bare?: boolean;
  /** When present, the stage participates in the global figure interactions. */
  figure?: FigureDescriptor;
  /** `hover` enlarges only (a select option must keep its click); `full` also opens the viewer. */
  interaction?: "hover" | "full";
}) {
  // The SVG carries its own viewBox, and CSS width/height override the
  // root's width/height *attributes* — so `h-full w-full` on a container
  // with a definite size fits the drawing exactly, preserving its aspect
  // ratio (the default `preserveAspectRatio` does that math). `contain`
  // keeps small drawings at their natural size instead of inflating them.
  const fitClass = fit === "fill"
    ? "[&>svg]:h-full [&>svg]:w-full"
    : "[&>svg]:max-h-full [&>svg]:max-w-full [&>svg]:h-auto [&>svg]:w-auto";
  const stage = (
    <div
      role="img"
      aria-label={label}
      className={`${bare
        ? "flex items-center justify-center overflow-hidden"
        : "flex items-center justify-center overflow-hidden rounded-md border border-border/60 bg-background p-2"} ${fitClass} ${className ?? ""}`}
      dangerouslySetInnerHTML={{ __html: svg.body }}
    />
  );
  if (!figure) return stage;
  return (
    <FigureInteractions figure={figure} mode={interaction === "full" ? "full" : "hover"}>
      {stage}
    </FigureInteractions>
  );
}

/**
 * The one renderer for declared rich content.
 *
 * Three possible outcomes, driven by the session data rather than guessed:
 * the rendered asset (themed for diagrams), an author's parse error shown
 * next to the source, or — when this build simply has no asset for the
 * content — the source itself as a code block. The last one is a capability
 * fallback, not an error, and is deliberately unstyled as such. A `mini`
 * with no asset renders nothing at all: a select option that cannot show
 * its figure should look like a plain option, not like a broken one.
 *
 * Every figure-bearing variant is a live thing: hovering enlarges it through
 * the hover layer, and (except for `mini`, whose click belongs to the select)
 * clicking opens the fullscreen viewer with copy/download — the same feel
 * everywhere a figure appears.
 */
export function RichSurface({
  content,
  variant,
  title,
  interactive = true,
}: {
  content: RichContent;
  variant: RichVariant;
  /** Names the figure in the fullscreen viewer and its aria-label. */
  title?: string;
  /** `false` keeps the figure fully inert. */
  interactive?: boolean;
}) {
  const { resolve } = useRichAssets();
  const { theme } = useTheme();
  const asset = resolve(content.type, content.source);

  if (!asset) {
    if (variant === "mini") return null;
    return <SourceFallback content={content} variant={variant} />;
  }
  if (asset.kind === "failed") {
    if (variant === "mini") return null;
    return (
      <div
        className={`space-y-1.5 rounded-md border border-destructive/30 bg-destructive/5 p-2.5 ${
          variant === "thumb" ? "w-52" : ""
        }`}
      >
        <FailedMessage message={asset.message} />
        <SourceFallback content={content} variant={variant} />
      </div>
    );
  }

  if (asset.kind === "html") {
    // Sanitised engine-side (ammonia) before it ever reached the wire; the
    // same trust the ui_ast itself gets.
    return (
      <div
        className={`prose prose-sm dark:prose-invert max-w-none text-sm ${
          variant === "viewer" ? "min-h-0" : ""
        }`}
        dangerouslySetInnerHTML={{ __html: asset.html }}
      />
    );
  }

  const svg = theme === "dark" && asset.kind === "diagram"
    ? asset.dark
    : asset.kind === "diagram"
    ? asset.light
    : asset.svg;
  const label = title ?? content.type;
  const figure: FigureDescriptor = { svg, title: label, content };

  return (
    <VariantStage
      svg={svg}
      variant={variant}
      label={label}
      figure={interactive ? figure : undefined}
    />
  );
}

/** The box each variant draws into; the drawing fits inside it. */
function VariantStage({
  svg,
  variant,
  label,
  figure,
}: {
  svg: RenderedSvg;
  variant: RichVariant;
  label: string;
  figure?: FigureDescriptor;
}) {
  switch (variant) {
    // A select row is ~2rem tall and its click selects: the mini shrinks to
    // fit, drops the frame, and keeps only the hover enlargement.
    case "mini":
      return (
        <DiagramStage
          svg={svg}
          label={label}
          fit="contain"
          bare
          className="h-7 w-12 shrink-0"
          figure={figure}
          interaction="hover"
        />
      );
    case "thumb":
      return (
        <DiagramStage
          svg={svg}
          label={label}
          fit="contain"
          className="h-20 w-40"
          figure={figure}
        />
      );
    case "block":
      return (
        <DiagramStage
          svg={svg}
          label={label}
          fit="fill"
          className="h-64 w-full"
          figure={figure}
        />
      );
    case "viewer":
      return (
        <DiagramStage svg={svg} label={label} fit="fill" className="h-full w-full min-h-0" />
      );
  }
}

/**
 * The interactions every visible figure shares, mounted by `DiagramStage`
 * itself: hover enlarges, click opens the fullscreen viewer, keyboard does
 * both. Progressive by construction — the contexts default to no-ops, so a
 * figure rendered outside a provider (a test, a future surface) is a plain
 * picture.
 */
function FigureInteractions({
  figure,
  mode,
  children,
}: {
  figure: FigureDescriptor;
  mode: "hover" | "full";
  children: React.ReactNode;
}) {
  const { t } = useI18n();
  const hover = useHoverPreview();
  const overlay = useOverlaySafe();
  const openViewer = () =>
    overlay?.open({
      title: figure.title ?? t("Figure"),
      wide: true,
      content: () => <FigureViewer figure={figure} />,
    });
  return (
    <span
      className={mode === "full" ? "inline-flex cursor-zoom-in" : "inline-flex"}
      onMouseEnter={(event) => hover.show(figure, event.currentTarget.getBoundingClientRect())}
      onMouseLeave={() => hover.hide()}
      onFocus={(event) => hover.show(figure, event.currentTarget.getBoundingClientRect())}
      onBlur={() => hover.hide()}
      onClick={mode === "full" ? openViewer : undefined}
      onKeyDown={(event) => {
        if (mode === "full" && (event.key === "Enter" || event.key === " ")) {
          openViewer();
        }
      }}
      tabIndex={mode === "full" && overlay ? 0 : undefined}
    >
      {children}
    </span>
  );
}

/** The fullscreen viewer: the figure filling a big stage, plus its source. */
export function FigureViewer({ figure }: { figure: FigureDescriptor }) {
  const { t } = useI18n();
  const { resolve } = useRichAssets();
  // Declared content renders live (and re-themes) through the normal path;
  // a figure with no session asset — the editor's on-the-fly preview — shows
  // the svg snapshot it was opened with.
  const asset = figure.content
    ? resolve(figure.content.type, figure.content.source)
    : undefined;
  const live = asset && asset.kind !== "failed";
  const source = figure.content?.source;
  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="min-h-0 flex-1">
        {live && figure.content
          ? <RichSurface content={figure.content} variant="viewer" interactive={false} />
          : <DiagramStage
            svg={figure.svg}
            label={figure.title ?? t("Figure")}
            fit="fill"
            className="h-full w-full min-h-0"
          />}
      </div>
      {source !== undefined && (
        <details className="shrink-0">
          <summary className="cursor-pointer text-xs text-muted-foreground hover:text-foreground">
            {t("Mermaid / SVG source")}
          </summary>
          <pre className="mt-2 max-h-32 overflow-auto rounded-md bg-muted p-2 text-xs">
            {source}
          </pre>
        </details>
      )}
      <div className="flex shrink-0 gap-2">
        {source !== undefined && (
          <button
            type="button"
            className="rounded-md border border-border px-2.5 py-1 text-xs hover:bg-muted"
            onClick={() => navigator.clipboard.writeText(source)}
          >
            {t("Copy source")}
          </button>
        )}
        <button
          type="button"
          className="rounded-md border border-border px-2.5 py-1 text-xs hover:bg-muted"
          onClick={() => {
            const body = live && asset && (asset.kind === "diagram" || asset.kind === "svg")
              ? asset.kind === "diagram" ? asset.light.body : asset.svg.body
              : figure.svg.body;
            const url = URL.createObjectURL(
              new Blob([body], { type: "image/svg+xml" }),
            );
            const anchor = document.createElement("a");
            anchor.href = url;
            anchor.download = "figure.svg";
            anchor.click();
            URL.revokeObjectURL(url);
          }}
        >
          {t("Download SVG")}
        </button>
      </div>
    </div>
  );
}

function FailedMessage({ message }: { message: string }) {
  const { t } = useI18n();
  return (
    <p className="text-xs font-medium text-destructive">
      {t("This figure could not be rendered: {message}", { message })}
    </p>
  );
}

/** The declared source, shown as a code block. */
function SourceFallback({
  content,
  variant,
}: {
  content: RichContent;
  variant: RichVariant;
}) {
  return (
    <pre
      className={`overflow-auto rounded-md bg-muted p-2 font-mono text-xs text-muted-foreground ${
        variant === "mini"
          ? "hidden"
          : variant === "thumb"
          ? "max-h-20 max-w-52"
          : "max-h-64"
      }`}
    >
      {content.source}
    </pre>
  );
}

/**
 * `useOverlay` throws outside its provider; a figure should survive a future
 * surface that forgot the overlay more than it should crash there.
 */
function useOverlaySafe() {
  try {
    return useOverlay();
  } catch {
    return null;
  }
}
