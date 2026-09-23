import { describe, expect, it } from "vitest";
import { computeFloatingPosition } from "./floatingPosition";

// A trigger sitting in the middle of a 1280x800 viewport, a 300x200 preview.
const TRIGGER = { x: 600, y: 400, width: 120, height: 32 };
const CONTENT = { width: 300, height: 200 };
const VIEWPORT = { width: 1280, height: 800 };

describe("computeFloatingPosition", () => {
  it("prefers below with the default gap", () => {
    const p = computeFloatingPosition(TRIGGER, CONTENT, VIEWPORT);
    expect(p.placement).toBe("bottom");
    expect(p.y).toBe(TRIGGER.y + TRIGGER.height + 8);
    expect(p.scale).toBe(1);
  });

  it("flips above when the space below is too tight", () => {
    const nearBottom = { ...TRIGGER, y: 700 };
    const p = computeFloatingPosition(nearBottom, CONTENT, VIEWPORT);
    expect(p.placement).toBe("top");
    expect(p.y).toBe(nearBottom.y - 8 - CONTENT.height);
  });

  it("honours an explicit preferred side when both fit", () => {
    const p = computeFloatingPosition(TRIGGER, CONTENT, VIEWPORT, { prefer: "top" });
    expect(p.placement).toBe("top");
  });

  it("keeps the preferred side when it fits even if the other is roomier", () => {
    // Near the bottom, `top` fits and `bottom` does not matter: the request
    // wins when it can be honoured.
    const nearBottom = { ...TRIGGER, y: 700 };
    const p = computeFloatingPosition(nearBottom, CONTENT, VIEWPORT, { prefer: "top" });
    expect(p.placement).toBe("top");
  });

  it("centres on the trigger when there is room", () => {
    const p = computeFloatingPosition(TRIGGER, CONTENT, VIEWPORT);
    expect(p.x).toBe(TRIGGER.x + TRIGGER.width / 2 - CONTENT.width / 2);
  });

  it("shifts left near the right edge instead of overflowing", () => {
    const nearRight = { ...TRIGGER, x: 1200 };
    const p = computeFloatingPosition(nearRight, CONTENT, VIEWPORT);
    expect(p.x).toBe(VIEWPORT.width - 8 - CONTENT.width);
  });

  it("shifts right near the left edge instead of overflowing", () => {
    const nearLeft = { ...TRIGGER, x: 10 };
    const p = computeFloatingPosition(nearLeft, CONTENT, VIEWPORT);
    expect(p.x).toBe(8);
  });

  it("handles the top-left corner: flip up, clamp right", () => {
    const corner = { x: 2, y: 2, width: 80, height: 24 };
    const p = computeFloatingPosition(corner, CONTENT, VIEWPORT);
    expect(p.placement).toBe("bottom");
    expect(p.x).toBe(8);
    expect(p.y).toBe(corner.y + corner.height + 8);
  });

  it("handles the bottom-right corner: flip above, clamp left", () => {
    const corner = { x: 1180, y: 760, width: 80, height: 24 };
    const p = computeFloatingPosition(corner, CONTENT, VIEWPORT);
    expect(p.placement).toBe("top");
    expect(p.x).toBe(VIEWPORT.width - 8 - CONTENT.width);
    expect(p.y).toBe(corner.y - 8 - CONTENT.height);
  });

  it("never places the content above the viewport top, even after a flip", () => {
    // A trigger near the bottom asked to open upward with content taller than
    // the space above: the flip happens, then the y clamp keeps the box
    // inside instead of letting it poke out over the top edge.
    const nearBottom = { x: 600, y: 700, width: 120, height: 32 };
    const tall = { width: 300, height: 700 };
    const p = computeFloatingPosition(nearBottom, tall, VIEWPORT, { prefer: "top" });
    expect(p.placement).toBe("top");
    expect(p.y).toBe(8);
  });

  it("never places the content below the viewport bottom", () => {
    const nearBottom = { x: 600, y: 700, width: 120, height: 60 };
    const p = computeFloatingPosition(nearBottom, CONTENT, VIEWPORT, {
      prefer: "bottom",
    });
    // "bottom" wins the preference when the opposite side is no roomier, and
    // the clamp shifts the box back inside instead of overflowing.
    expect(p.y + CONTENT.height).toBeLessThanOrEqual(VIEWPORT.height - 8);
  });

  it("reports a shrink scale when even the viewport cannot fit the content", () => {
    const huge = { width: 3000, height: 2000 };
    const p = computeFloatingPosition(TRIGGER, huge, VIEWPORT);
    expect(p.scale).toBeLessThan(1);
    expect((huge.height * p.scale)).toBeCloseTo(VIEWPORT.height - 16);
  });

  it("never scales when everything fits", () => {
    const p = computeFloatingPosition(TRIGGER, CONTENT, VIEWPORT);
    expect(p.scale).toBe(1);
  });
});
