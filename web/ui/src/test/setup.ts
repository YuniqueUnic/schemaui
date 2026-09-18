import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}

globalThis.ResizeObserver = ResizeObserverMock as typeof ResizeObserver;

HTMLElement.prototype.scrollIntoView = () => {};

// jsdom has no pointer capture. The colour picker drags with it, so without a
// stub every drag test fails on a missing method rather than on the behaviour
// under test. The stub records which element holds the capture, which is what
// the component actually reads back.
const captured = new WeakMap<Element, Set<number>>();

HTMLElement.prototype.setPointerCapture = function (pointerId: number) {
  const held = captured.get(this) ?? new Set<number>();
  held.add(pointerId);
  captured.set(this, held);
};

HTMLElement.prototype.releasePointerCapture = function (pointerId: number) {
  captured.get(this)?.delete(pointerId);
};

HTMLElement.prototype.hasPointerCapture = function (pointerId: number) {
  return captured.get(this)?.has(pointerId) ?? false;
};

// `getBoundingClientRect` is all zeroes in jsdom, so every pointer position
// would normalise to 0/0 and the colour maths would look broken. A fixed box
// gives the tests a known coordinate space to aim at.
HTMLElement.prototype.getBoundingClientRect = function () {
  return {
    x: 0,
    y: 0,
    top: 0,
    left: 0,
    right: 200,
    bottom: 100,
    width: 200,
    height: 100,
    toJSON: () => ({}),
  } as DOMRect;
};

afterEach(() => {
  cleanup();
});
