import { createContext, useCallback, useContext, useMemo, useState } from 'react';
import { createPortal } from 'react-dom';

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

  const close = useCallback(() => {
    setOptions(null);
  }, []);

  const open = useCallback((next: OverlayOptions) => {
    setOptions(next);
  }, []);

  const portal = useMemo(() => {
    if (!options || typeof document === 'undefined') {
      return null;
    }
    return createPortal(
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/70 px-4 py-8">
        <div className="relative w-full max-w-3xl rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900">
          <header className="flex items-start justify-between gap-4">
            <div>
              <p className="text-lg font-semibold text-slate-900 dark:text-slate-100">
                {options.title || 'Details'}
              </p>
              {options.description ? (
                <p className="text-sm text-slate-600 dark:text-slate-300">{options.description}</p>
              ) : null}
            </div>
            <button
              type="button"
              onClick={close}
              className="rounded-full border border-slate-300 px-3 py-1 text-xs font-semibold text-slate-600 hover:border-rose-400 hover:text-rose-500 dark:border-slate-700 dark:text-slate-300"
            >
              Close
            </button>
          </header>
          <div className="mt-4 max-h-[70vh] overflow-auto pr-1 text-sm text-slate-900 dark:text-slate-100">
            {options.content(close)}
          </div>
        </div>
      </div>,
      document.body,
    );
  }, [options, close]);

  return (
    <OverlayContext.Provider value={{ open, close }}>
      {children}
      {portal}
    </OverlayContext.Provider>
  );
}

export function useOverlay() {
  const ctx = useContext(OverlayContext);
  if (!ctx) {
    throw new Error('useOverlay must be used within OverlayProvider');
  }
  return ctx;
}
