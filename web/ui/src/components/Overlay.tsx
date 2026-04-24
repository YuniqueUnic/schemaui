/* eslint-disable react-refresh/only-export-components */

import { createContext, useCallback, useContext, useRef, useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface OverlayOptions {
  title?: string;
  description?: string;
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
          className="max-w-3xl max-h-[85vh] overflow-hidden"
          aria-describedby={top?.description ? undefined : "overlay-fallback-description"}
        >
          <DialogHeader>
            <div className="flex items-center justify-between gap-3">
              <DialogTitle>{top?.title || "Details"}</DialogTitle>
              {stack.length > 1 && (
                <span className="text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
                  Layer {stack.length}
                </span>
              )}
            </div>
            {top?.description && (
              <DialogDescription>{top.description}</DialogDescription>
            )}
            {!top?.description && (
              <DialogDescription id="overlay-fallback-description" className="sr-only">
                Nested editor overlay
              </DialogDescription>
            )}
          </DialogHeader>
          <div className="max-h-[60vh] min-h-[16rem] overflow-y-auto px-1 py-1 sm:px-2">
            {stack.map((frame, index) => (
              <div
                key={frame.id}
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
