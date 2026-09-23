/**
 * Viewport-aware positioning for floating layers.
 *
 * A pure function on purpose: the popover (`ui/popover.tsx`) measures real
 * DOM rects and applies the result, and this decides *where* — which keeps
 * the collision policy testable across every edge and corner without a DOM.
 * The algorithm is the documented Floating-UI subset (prefer a side, flip to
 * the opposite side when it does not fit, shift along the cross axis, clamp
 * to the viewport, shrink when even the clamped box cannot fit).
 */

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Viewport {
  width: number;
  height: number;
}

export type Placement = "top" | "bottom";

export interface FloatingPosition {
  x: number;
  y: number;
  /** Which side of the trigger the content ended up on. */
  placement: Placement;
  /**
   * 1 when the content fits, otherwise the uniform scale (0..1) that makes
   * the clamped content fit the viewport — applied by the caller as a CSS
   * transform so an oversized preview shrinks instead of overflowing.
   */
  scale: number;
}

export interface FloatingOptions {
  /** Gap between the trigger and the content, in pixels. */
  gap?: number;
  /** Minimum distance kept from the viewport edges. */
  margin?: number;
  /** Which side to try first; the other side is the fallback. Default bottom. */
  prefer?: Placement;
}

const DEFAULT_GAP = 8;
const DEFAULT_MARGIN = 8;

export function computeFloatingPosition(
  trigger: Rect,
  content: { width: number; height: number },
  viewport: Viewport,
  options: FloatingOptions = {},
): FloatingPosition {
  const gap = options.gap ?? DEFAULT_GAP;
  const margin = options.margin ?? DEFAULT_MARGIN;
  const prefer = options.prefer ?? "bottom";

  const spaceAbove = trigger.y - gap - margin;
  const spaceBelow = viewport.height - (trigger.y + trigger.height) - gap - margin;
  const placement: Placement = prefer === "bottom"
    ? (content.height <= spaceBelow || spaceBelow >= spaceAbove ? "bottom" : "top")
    : (content.height <= spaceAbove || spaceAbove >= spaceBelow ? "top" : "bottom");

  // Cross axis: centre on the trigger, then shift back inside the viewport.
  let x = trigger.x + trigger.width / 2 - content.width / 2;
  x = Math.min(Math.max(x, margin), Math.max(margin, viewport.width - margin - content.width));

  // Same clamp on the main axis: a flip can still land off-screen when the
  // content is taller than the space above *and* below, so keep the chosen
  // placement but shift the box inside.
  let y = placement === "bottom"
    ? trigger.y + trigger.height + gap
    : trigger.y - gap - content.height;
  y = Math.min(Math.max(y, margin), Math.max(margin, viewport.height - margin - content.height));

  // Last resort: the viewport itself is smaller than the content (after the
  // horizontal clamp above there is no placement that fits) — keep the
  // position, report the scale that makes it fit.
  const availableHeight = viewport.height - margin * 2;
  const availableWidth = viewport.width - margin * 2;
  const scale = content.height > availableHeight || content.width > availableWidth
    ? Math.min(availableHeight / content.height, availableWidth / content.width, 1)
    : 1;

  return { x, y, placement, scale };
}
