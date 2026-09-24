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
import { computeFloatingPosition, type FloatingPosition } from "../../lib/floatingPosition";
import { DiagramStage, type FigureDescriptor } from "./RichSurface";
import { useI18n } from "../../i18n";

interface HoverRequest {
  figure: FigureDescriptor;
  trigger: DOMRect;
}

interface HoverPreviewContextValue {
  /** Show the preview for `figure`, anchored to `trigger`'s rect. */
  show: (figure: FigureDescriptor, trigger: DOMRect) => void;
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
 * preview ends up. It speaks `FigureDescriptor`, so any figure anywhere —
 * a declared asset or the editor's on-the-fly render — enlarges the same
 * way. Positioned by the pure `computeFloatingPosition` (below-first, flip,
 * shift, clamp, shrink), re-measured on scroll and resize. Keyboard users
 * get the same preview via focus, and Escape closes it; the layer never
 * takes focus itself, so the underlying control keeps working.
 */
export function HoverPreviewProvider({ children }: { children: ReactNode }) {
  const { t } = useI18n();
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

  const show = useCallback((figure: FigureDescriptor, trigger: DOMRect) => {
    clearTimers();
    triggerRef.current = trigger;
    timers.current.show = window.setTimeout(() => {
      setRequest({ figure, trigger });
      setPosition(null);
      setVisible(true);
    }, SHOW_DELAY_MS);
  }, []);

  const hide = useCallback(() => {
    clearTimers();
    timers.current.hide = window.setTimeout(() => {
      setVisible(false);
      setRequest(null);
      setPosition(null);
      triggerRef.current = null;
    }, HIDE_GRACE_MS);
  }, []);

  // Re-measure while visible; the measured content size feeds the collision
  // pass, so the position settles over two frames like the popover does.
  // (Resetting position lives in the show/hide transitions above — state
  // changes belong to the state machine, not to a reactive afterthought.)
  useEffect(() => {
    if (!visible) {
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
        setPosition(null);
        triggerRef.current = null;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [visible]);

  const value = useMemo(() => ({ show, hide }), [show, hide]);

  return (
    <HoverPreviewContext.Provider value={value}>
      {children}
      {visible && request?.figure &&
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
            <DiagramStage
              svg={request.figure.svg}
              label={request.figure.title ?? t("Figure")}
              fit="fill"
              className="h-[60vmin] w-[60vmin]"
            />
          </div>,
          document.body,
        )}
    </HoverPreviewContext.Provider>
  );
}

export function useHoverPreview(): HoverPreviewContextValue {
  return useContext(HoverPreviewContext);
}
