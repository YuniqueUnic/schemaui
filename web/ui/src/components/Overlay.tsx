/* eslint-disable react-refresh/only-export-components */

import { createContext, useCallback, useContext, useRef, useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useI18n } from "../i18n";

interface OverlayOptions {
  title?: string;
  description?: string;
  /**
   * A roomy stage for figures: the dialog grows to nearly the whole screen
   * on desktop and becomes truly fullscreen on a phone, so a diagram has
   * room instead of sitting tiny inside a tall empty frame.
   */
  wide?: boolean;
  content: (close: () => void) => React.ReactNode;
}

interface OverlayContextValue {
  open: (options: OverlayOptions) => void;
  close: () => void;
}

interface OverlayFrame extends OverlayOptions {
  id: number;
}

const OverlayContext = createContext<OverlayContextValue | null>(null);

export function OverlayProvider({ children }: { children: React.ReactNode }) {
  const { t } = useI18n();
  const [stack, setStack] = useState<OverlayFrame[]>([]);
  const nextId = useRef(1);

  const close = useCallback(() => {
    setStack((current) => current.slice(0, -1));
  }, []);

  const open = useCallback((next: OverlayOptions) => {
    setStack((current) => [
      ...current,
      {
        ...next,
        id: nextId.current++,
      },
    ]);
  }, []);

  const top = stack[stack.length - 1];

  return (
    <OverlayContext.Provider value={{ open, close }}>
      {children}
      <Dialog open={stack.length > 0} onOpenChange={(open) => !open && close()}>
        <DialogContent
          className={
            top?.wide
              ? // Fullscreen on a phone; near-fullscreen on anything bigger.
                "flex h-dvh w-screen max-w-none flex-col rounded-none border-0 p-4 sm:h-[88vh] sm:w-[min(96vw,1200px)] sm:rounded-lg sm:border sm:p-6"
              : "max-w-3xl max-h-[85vh] overflow-hidden"
          }
          aria-describedby={top?.description ? undefined : "overlay-fallback-description"}
        >
          <DialogHeader>
            <div className="flex items-center justify-between gap-3">
              <DialogTitle>{top?.title || t("Details")}</DialogTitle>
              {stack.length > 1 && (
                <span className="text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
                  {t("Layer {count}", { count: stack.length })}
                </span>
              )}
            </div>
            {top?.description && (
              <DialogDescription>{top.description}</DialogDescription>
            )}
            {!top?.description && (
              <DialogDescription id="overlay-fallback-description" className="sr-only">
                {t("Nested editor overlay")}
              </DialogDescription>
            )}
          </DialogHeader>
          <div
            className={
              top?.wide
                ? // The stage owns the leftover height; frames scroll within.
                  "min-h-0 flex-1 overflow-y-auto px-1 py-1 sm:px-2"
                : "max-h-[60vh] min-h-[16rem] overflow-y-auto px-1 py-1 sm:px-2"
            }
          >
            {stack.map((frame, index) => (
              <div
                key={frame.id}
                // `h-full` lets a wide frame's content own the stage height;
                // in the default dialog the parent's height is auto, so it is
                // harmless there.
                className="h-full"
                hidden={index !== stack.length - 1}
                aria-hidden={index !== stack.length - 1}
              >
                {frame.content(close)}
              </div>
            ))}
          </div>
        </DialogContent>
      </Dialog>
    </OverlayContext.Provider>
  );
}

export function useOverlay() {
  const ctx = useContext(OverlayContext);
  if (!ctx) {
    throw new Error("useOverlay must be used within OverlayProvider");
  }
  return ctx;
}
