/* eslint-disable react-refresh/only-export-components */

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import type { RichContent } from "../../types";
import { computeFloatingPosition, type FloatingPosition } from "../../lib/floatingPosition";
import { RichSurface } from "./RichSurface";

interface HoverRequest {
  content: RichContent;
  trigger: DOMRect;
}

interface HoverPreviewContextValue {
  /** Show the preview for `content`, anchored to `trigger`'s rect. */
  show: (content: RichContent, trigger: DOMRect) => void;
  /** Ask to hide; a short grace period absorbs thumb-to-preview travel. */
  hide: () => void;
}

const HoverPreviewContext = createContext<HoverPreviewContextValue>({
  show: () => {},
  hide: () => {},
});

const SHOW_DELAY_MS = 180;
const HIDE_GRACE_MS = 120;

/**
 * The one floating layer every rich thumbnail enlarges into.
 *
 * A singleton rather than a popover per row: one portal means one z-index
 * story and one repositioning loop, and a row never has to know where its
 * preview ends up. Positioned by the pure `computeFloatingPosition`
 * (below-first, flip, shift, clamp, shrink), re-measured on scroll and
 * resize. Keyboard users get the same preview via focus, and Escape closes
 * it; the layer never takes focus itself, so the underlying control keeps
 * working.
 */
export function HoverPreviewProvider({ children }: { children: ReactNode }) {
  const [request, setRequest] = useState<HoverRequest | null>(null);
  const [visible, setVisible] = useState(false);
  const [position, setPosition] = useState<FloatingPosition | null>(null);
  const timers = useRef<{ show?: number; hide?: number }>({});
  // The preview follows the anchor rect while it is open: scrolling the form
  // under a pinned cursor would otherwise leave the preview behind.
  const triggerRef = useRef<DOMRect | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);

  const clearTimers = () => {
    if (timers.current.show !== undefined) window.clearTimeout(timers.current.show);
    if (timers.current.hide !== undefined) window.clearTimeout(timers.current.hide);
    timers.current = {};
  };

  const show = useCallback((content: RichContent, trigger: DOMRect) => {
    clearTimers();
    triggerRef.current = trigger;
    timers.current.show = window.setTimeout(() => {
      setRequest({ content, trigger });
      setVisible(true);
    }, SHOW_DELAY_MS);
  }, []);

  const hide = useCallback(() => {
    clearTimers();
    timers.current.hide = window.setTimeout(() => {
      setVisible(false);
      setRequest(null);
      triggerRef.current = null;
    }, HIDE_GRACE_MS);
  }, []);

  // Re-measure while visible; the measured content size feeds the collision
  // pass, so the position settles over two frames like the popover does.
  useEffect(() => {
    if (!visible) {
      setPosition(null);
      return;
    }
    const measure = () => {
      const trigger = triggerRef.current;
      const element = contentRef.current;
      if (!trigger || !element) {
        return;
      }
      setPosition(
        computeFloatingPosition(
          { x: trigger.x, y: trigger.y, width: trigger.width, height: trigger.height },
          { width: element.offsetWidth, height: element.offsetHeight },
          { width: window.innerWidth, height: window.innerHeight },
        ),
      );
    };
    measure();
    const raf = window.requestAnimationFrame(measure);
    window.addEventListener("resize", measure);
    window.addEventListener("scroll", measure, true);
    return () => {
      window.cancelAnimationFrame(raf);
      window.removeEventListener("resize", measure);
      window.removeEventListener("scroll", measure, true);
    };
  }, [visible, request]);

  // Escape closes; the layer itself is never focusable, so focus stays on
  // the trigger that opened the preview.
  useEffect(() => {
    if (!visible) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        clearTimers();
        setVisible(false);
        setRequest(null);
        triggerRef.current = null;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  const value = useMemo(() => ({ show, hide }), [show, hide]);

  return (
    <HoverPreviewContext.Provider value={value}>
      {children}
      {visible && request?.content &&
        createPortal(
          <div
            ref={contentRef}
            // `pointer-events-none` while unpositioned so a preview that has
            // not found its place yet cannot steal a hover from the row.
            className={`pointer-events-none fixed z-50 transition-opacity duration-150 ${
              position ? "opacity-100" : "opacity-0"
            }`}
            style={
              position
                ? {
                    left: position.x,
                    top: position.y,
                    transform: position.scale < 1
                      ? `scale(${position.scale})`
                      : undefined,
                    transformOrigin: "top left",
                  }
                : { visibility: "hidden" }
            }
          >
            <RichSurface content={request.content} variant="preview" />
          </div>,
          document.body,
        )}
    </HoverPreviewContext.Provider>
  );
}

export function useHoverPreview(): HoverPreviewContextValue {
  return useContext(HoverPreviewContext);
}
