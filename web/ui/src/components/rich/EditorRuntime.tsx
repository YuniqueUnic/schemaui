/* eslint-disable react-refresh/only-export-components */

import {
  createContext,
  useContext,
  useMemo,
  type ReactNode,
} from "react";
import type { Capability } from "../../types";
import type { RenderContentInput, RenderContentResult, SchemaUiTransport } from "../../transport/types";

interface EditorRuntimeContextValue {
  /**
   * Render a diagram on demand. `null` when this session offers no renderer
   * — either the transport has no `renderContent` or the session's
   * `capabilities` lack `content_render`. Both mean "don't mount a preview".
   */
  renderContent: ((input: RenderContentInput) => Promise<RenderContentResult>) | null;
}

const EditorRuntimeContext = createContext<EditorRuntimeContextValue>({
  renderContent: null,
});

/**
 * Hands the live-preview editor its on-demand renderer.
 *
 * Mounted once in `App`, where the transport and the session capabilities
 * already live; a field two renderers deep consumes it without either being
 * threaded through props it does not otherwise need.
 */
export function EditorRuntimeProvider({
  transport,
  capabilities,
  children,
}: {
  transport: SchemaUiTransport;
  capabilities?: Capability[];
  children: ReactNode;
}) {
  const value = useMemo<EditorRuntimeContextValue>(() => {
    const offered = capabilities?.includes("content_render") ?? false;
    return {
      renderContent: offered && transport.renderContent
        ? (input) => transport.renderContent!(input)
        : null,
    };
  }, [transport, capabilities]);

  return (
    <EditorRuntimeContext.Provider value={value}>
      {children}
    </EditorRuntimeContext.Provider>
  );
}

export function useEditorRuntime(): EditorRuntimeContextValue {
  return useContext(EditorRuntimeContext);
}
