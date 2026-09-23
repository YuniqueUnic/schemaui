/* eslint-disable react-refresh/only-export-components */

import { createContext, useContext, useMemo, type ReactNode } from "react";
import type { RichAsset } from "../../types";
import { richContentId } from "../../lib/richId";

interface RichAssetsContextValue {
  /**
   * The rendered asset for a declared content, or `null` when this build did
   * not render it — either the schema never declared it (the normal case for
   * plain schemas) or the engine lacks the capability. Both mean "fall back
   * to the source", not "error".
   */
  resolve: (kind: "mermaid" | "svg" | "markdown", source: string) => RichAsset | null;
}

const RichAssetsContext = createContext<RichAssetsContextValue>({
  resolve: () => null,
});

/**
 * Exposes the session's rendered rich content to everything below it.
 *
 * Mounted once, around the form: the map is computed once per session on the
 * engine side and never changes, so no per-figure lookups need state.
 */
export function RichAssetsProvider({
  assets,
  children,
}: {
  assets?: Record<string, RichAsset> | null;
  children: ReactNode;
}) {
  const value = useMemo<RichAssetsContextValue>(() => {
    if (!assets) {
      return { resolve: () => null };
    }
    return {
      resolve: (kind, source) => assets[richContentId(kind, source)] ?? null,
    };
  }, [assets]);

  return (
    <RichAssetsContext.Provider value={value}>
      {children}
    </RichAssetsContext.Provider>
  );
}

export function useRichAssets(): RichAssetsContextValue {
  return useContext(RichAssetsContext);
}
