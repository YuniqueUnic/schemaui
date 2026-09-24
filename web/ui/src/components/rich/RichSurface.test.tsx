import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { RichSurface } from "./RichSurface";
import { RichAssetsProvider } from "./RichAssetsProvider";
import { HoverPreviewProvider } from "./HoverPreviewLayer";
import { ThemeProvider } from "../../theme";
import { richContentId } from "../../lib/richId";
import type { RichContent } from "../../types";

const SVG_SOURCE = `<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/></svg>`;
const figure: RichContent = { type: "svg", source: SVG_SOURCE };

const ASSETS = {
  [richContentId("svg", SVG_SOURCE)]: {
    kind: "svg" as const,
    svg: { body: "<svg viewBox='0 0 24 24'></svg>", width: 24, height: 24 },
  },
};

function rendered(
  content: RichContent,
  assets?: Record<string, import("../../types").RichAsset> | null,
  variant: "block" | "thumb" | "mini" = "block",
) {
  return render(
    <ThemeProvider>
      <RichAssetsProvider assets={assets}>
        <RichSurface content={content} variant={variant} />
      </RichAssetsProvider>
    </ThemeProvider>,
  );
}

describe("RichSurface", () => {
  it("renders a sanitised svg asset as live markup", () => {
    const view = rendered(figure, {
      [richContentId("svg", SVG_SOURCE)]: {
        kind: "svg",
        svg: { body: "<svg viewBox='0 0 24 24'></svg>", width: 24, height: 24 },
      },
    });
    expect(view.container.querySelector("svg")).toBeTruthy();
  });

  it("falls back to the source when no asset exists — quietly, not as an error", () => {
    rendered(figure, null);
    expect(screen.getByText(SVG_SOURCE)).toBeTruthy();
    expect(screen.queryByText(/could not be rendered/i)).toBeNull();
  });

  it("shows a parse failure next to the source", () => {
    rendered(figure, {
      [richContentId("svg", SVG_SOURCE)]: { kind: "failed", message: "not an <svg> document" },
    });
    expect(screen.getByText(/not an <svg> document/)).toBeTruthy();
    expect(screen.getByText(SVG_SOURCE)).toBeTruthy();
  });

  it("mounts sanitised html from a markdown asset", () => {
    const source = "# Notes";
    rendered({ type: "markdown", source }, {
      [richContentId("markdown", source)]: { kind: "html", html: "<h1>Notes</h1>" },
    });
    expect(screen.getByText("Notes").tagName).toBe("H1");
  });

  it("wraps thumb and block variants as interactive figures", () => {
    for (const variant of ["thumb", "block"] as const) {
      const { container, unmount } = rendered(figure, ASSETS, variant);
      expect(container.querySelector("span.cursor-zoom-in")).toBeTruthy();
      unmount();
    }
  });
});

describe("RichSurface interactions", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });
  afterEach(() => {
    vi.useRealTimers();
    localStorage.removeItem("schemaui-locale");
  });

  it("a mini keeps only the hover enlargement — its click belongs to the select", async () => {
    const { container } = render(
      <ThemeProvider>
        <HoverPreviewProvider>
          <RichAssetsProvider assets={ASSETS}>
            <RichSurface content={figure} variant="mini" title="Mini figure" />
          </RichAssetsProvider>
        </HoverPreviewProvider>
      </ThemeProvider>,
    );
    // No click affordance…
    expect(container.querySelector("span.cursor-zoom-in")).toBeNull();
    // …but hovering still enlarges through the shared layer.
    fireEvent.mouseEnter(container.querySelector("span")!);
    await vi.advanceTimersByTimeAsync(250);
    const previews = document.body.querySelectorAll('[role="img"]');
    expect(previews.length).toBeGreaterThan(0);
  });
});
