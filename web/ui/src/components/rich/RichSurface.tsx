import { useOverlay } from "../Overlay";
import { useTheme } from "../../theme";
import { useRichAssets } from "./RichAssetsProvider";
import { useHoverPreview } from "./HoverPreviewLayer";
import type { RenderedSvg, RichContent } from "../../types";

/**
 * How much room a figure gets, and how it is allowed to use it:
 *
 * - `mini` — inside a select option; shrinks only, never interactive;
 * - `thumb` — an option's figure inside a compact control row;
 * - `block` — a figure declared on the node itself, interactive like a thumb;
 * - `preview` — the hover layer's enlargement;
 * - `viewer` — the fullscreen viewer stage, where the figure fills the room
 *   it is given (up *and* down) instead of sitting at its intrinsic size.
 */
export type RichVariant = "mini" | "thumb" | "block" | "preview" | "viewer";

/** Rendered SVG fitted into a box the caller sizes. */
export function DiagramStage({
  svg,
  label,
  fit = "contain",
  className,
}: {
  svg: RenderedSvg;
  label: string;
  /** `fill` lets the drawing scale up to the box; `contain` only shrinks. */
  fit?: "fill" | "contain";
  className?: string;
}) {
  // The SVG carries its own viewBox, and CSS width/height override the
  // root's width/height *attributes* — so `h-full w-full` on a container
  // with a definite size fits the drawing exactly, preserving its aspect
  // ratio (the default `preserveAspectRatio` does that math). `contain`
  // keeps small drawings at their natural size instead of inflating them.
  const fitClass = fit === "fill"
    ? "[&>svg]:h-full [&>svg]:w-full"
    : "[&>svg]:max-h-full [&>svg]:max-w-full [&>svg]:h-auto [&>svg]:w-auto";
  return (
    <div
      role="img"
      aria-label={label}
      className={`flex items-center justify-center overflow-hidden rounded-md border border-border/60 bg-background p-2 ${fitClass} ${className ?? ""}`}
      dangerouslySetInnerHTML={{ __html: svg.body }}
    />
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
 * Every figure-bearing variant except `mini` is a live thing: hovering
 * enlarges it through the hover layer and clicking opens the fullscreen
 * viewer with copy/download — the same feel everywhere a figure appears.
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
  /** `false` keeps the figure inert (inside a select option, say). */
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
        <p className="text-xs font-medium text-destructive">
          This figure could not be rendered: {asset.message}
        </p>
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
  const body = <VariantStage svg={svg} variant={variant} label={title ?? content.type} />;

  if (interactive && (variant === "thumb" || variant === "block")) {
    return (
      <InteractiveFigure content={content} title={title}>
        {body}
      </InteractiveFigure>
    );
  }
  return body;
}

/** The box each variant draws into; the drawing fits inside it. */
function VariantStage({
  svg,
  variant,
  label,
}: {
  svg: RenderedSvg;
  variant: RichVariant;
  label: string;
}) {
  switch (variant) {
    case "mini":
      return <DiagramStage svg={svg} label={label} fit="contain" className="h-10 w-16 shrink-0" />;
    case "thumb":
      return <DiagramStage svg={svg} label={label} fit="contain" className="h-20 w-40" />;
    case "block":
      return <DiagramStage svg={svg} label={label} fit="fill" className="h-64 w-full" />;
    case "preview":
      return <DiagramStage svg={svg} label={label} fit="fill" className="h-[60vmin] w-[60vmin]" />;
    case "viewer":
      return <DiagramStage svg={svg} label={label} fit="fill" className="h-full w-full min-h-0" />;
  }
}

/**
 * The interactions every visible figure shares: hover enlarges, click opens
 * the fullscreen viewer, keyboard does both.
 *
 * Progressive by construction: the context defaults are no-ops, so a figure
 * rendered outside a provider (a test, a future surface) is a plain picture.
 */
function InteractiveFigure({
  content,
  title,
  children,
}: {
  content: RichContent;
  title?: string;
  children: React.ReactNode;
}) {
  const hover = useHoverPreview();
  const overlay = useOverlaySafe();
  const openViewer = () =>
    overlay?.open({
      title: title ?? "Figure",
      wide: true,
      content: () => <RichViewer content={content} />,
    });
  return (
    <span
      className="inline-flex cursor-zoom-in"
      onMouseEnter={(event) => hover.show(content, event.currentTarget.getBoundingClientRect())}
      onMouseLeave={() => hover.hide()}
      onFocus={(event) => hover.show(content, event.currentTarget.getBoundingClientRect())}
      onBlur={() => hover.hide()}
      onClick={openViewer}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          openViewer();
        }
      }}
      tabIndex={overlay ? 0 : undefined}
    >
      {children}
    </span>
  );
}

/** The fullscreen viewer: the figure filling a big stage, plus its source. */
function RichViewer({ content }: { content: RichContent }) {
  const { resolve } = useRichAssets();
  const asset = resolve(content.type, content.source);
  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="min-h-0 flex-1">
        {asset && asset.kind !== "failed"
          ? <RichSurface content={content} variant="viewer" interactive={false} />
          : <SourceFallback content={content} variant="viewer" />}
      </div>
      <details className="shrink-0">
        <summary className="cursor-pointer text-xs text-muted-foreground hover:text-foreground">
          Mermaid / SVG source
        </summary>
        <pre className="mt-2 max-h-32 overflow-auto rounded-md bg-muted p-2 text-xs">
          {content.source}
        </pre>
      </details>
      <div className="flex shrink-0 gap-2">
        <button
          type="button"
          className="rounded-md border border-border px-2.5 py-1 text-xs hover:bg-muted"
          onClick={() => navigator.clipboard.writeText(content.source)}
        >
          Copy source
        </button>
        {asset && (asset.kind === "diagram" || asset.kind === "svg") && (
          <button
            type="button"
            className="rounded-md border border-border px-2.5 py-1 text-xs hover:bg-muted"
            onClick={() => {
              const svg = asset.kind === "diagram" ? asset.light : asset.svg;
              const url = URL.createObjectURL(
                new Blob([svg.body], { type: "image/svg+xml" }),
              );
              const anchor = document.createElement("a");
              anchor.href = url;
              anchor.download = "figure.svg";
              anchor.click();
              URL.revokeObjectURL(url);
            }}
          >
            Download SVG
          </button>
        )}
      </div>
    </div>
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
