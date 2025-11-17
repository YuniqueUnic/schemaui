import { createContext, useCallback, useContext, useState } from "react";
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

const OverlayContext = createContext<OverlayContextValue | null>(null);

export function OverlayProvider({ children }: { children: React.ReactNode }) {
  const [options, setOptions] = useState<OverlayOptions | null>(null);
  const [isOpen, setIsOpen] = useState(false);

  const close = useCallback(() => {
    setIsOpen(false);
    setTimeout(() => setOptions(null), 200); // Wait for animation
  }, []);

  const open = useCallback((next: OverlayOptions) => {
    setOptions(next);
    setIsOpen(true);
  }, []);

  return (
    <OverlayContext.Provider value={{ open, close }}>
      {children}
      <Dialog open={isOpen} onOpenChange={(open) => !open && close()}>
        <DialogContent className="max-w-3xl max-h-[85vh] overflow-hidden">
          <DialogHeader>
            <DialogTitle>{options?.title || "Details"}</DialogTitle>
            {options?.description && (
              <DialogDescription>{options.description}</DialogDescription>
            )}
          </DialogHeader>
          <div className="max-h-[60vh] overflow-y-auto pr-4">
            {options?.content(close)}
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
