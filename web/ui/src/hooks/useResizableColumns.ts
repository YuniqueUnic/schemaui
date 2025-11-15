import { useCallback, useEffect, useRef, useState } from 'react';
import type { PointerEvent as ReactPointerEvent } from 'react';

export interface ColumnSizes {
  nav: number;
  preview: number;
}

const LIMITS = {
  nav: { min: 220, max: 520 },
  preview: { min: 320, max: 720 },
};

type DragTarget = 'nav' | 'preview' | null;

export function useResizableColumns(initial: ColumnSizes) {
  const [sizes, setSizes] = useState<ColumnSizes>(initial);
  const dragTarget = useRef<DragTarget>(null);
  const dragOrigin = useRef<{ startX: number; width: number }>({
    startX: 0,
    width: 0,
  });

  const startDrag = useCallback((event: ReactPointerEvent, target: DragTarget) => {
    dragTarget.current = target;
    dragOrigin.current = {
      startX: event.clientX,
      width: target === 'nav' ? sizes.nav : sizes.preview,
    };
    (event.target as HTMLElement).setPointerCapture(event.pointerId);
    document.body.classList.add('select-none', 'cursor-col-resize');
  }, [sizes.nav, sizes.preview]);

  const stopDrag = useCallback(() => {
    dragTarget.current = null;
    document.body.classList.remove('select-none', 'cursor-col-resize');
  }, []);

  useEffect(() => {
    function handlePointerMove(event: PointerEvent) {
      const target = dragTarget.current;
      if (!target) return;
      const delta = event.clientX - dragOrigin.current.startX;
      if (target === 'nav') {
        const next = clamp(
          dragOrigin.current.width + delta,
          LIMITS.nav.min,
          LIMITS.nav.max,
        );
        setSizes((prev) => ({ ...prev, nav: next }));
      } else {
        const next = clamp(
          dragOrigin.current.width - delta,
          LIMITS.preview.min,
          LIMITS.preview.max,
        );
        setSizes((prev) => ({ ...prev, preview: next }));
      }
    }

    function handlePointerUp() {
      stopDrag();
    }

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    return () => {
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    };
  }, [stopDrag]);

  return { sizes, startDrag, stopDrag };
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}
