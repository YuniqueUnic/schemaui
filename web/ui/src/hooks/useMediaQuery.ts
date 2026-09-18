import { useCallback, useSyncExternalStore } from "react";

/**
 * Track a CSS media query.
 *
 * Reads through `useSyncExternalStore` rather than mirroring the query into
 * state from an effect. The effect version painted one frame with the wrong
 * answer before correcting itself, and it could miss a change that landed
 * between the first render and the subscription — which is exactly the window
 * this hook runs in, since the layout flips on it.
 */
export function useMediaQuery(query: string): boolean {
  const subscribe = useCallback(
    (onStoreChange: () => void) => {
      if (typeof window === "undefined") return () => {};
      const mql = window.matchMedia(query);
      mql.addEventListener("change", onStoreChange);
      return () => mql.removeEventListener("change", onStoreChange);
    },
    [query],
  );

  const getSnapshot = useCallback(
    () =>
      typeof window === "undefined" ? false : window.matchMedia(query).matches,
    [query],
  );

  return useSyncExternalStore(subscribe, getSnapshot, () => false);
}
