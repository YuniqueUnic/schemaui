import { useEffect, useMemo } from "react";
import { AppHeader } from "./components/AppHeader";
import { NodeRenderer } from "./components/NodeRenderer";
import { PreviewPane } from "./components/PreviewPane";
import { StatusBar } from "./components/StatusBar";
import { TreeView } from "./components/TreeView";
import { OverlayProvider } from "./components/Overlay";
import { ValidationErrorsDialog } from "./components/ValidationErrorsDialog";
import type { JsonValue, UiNode } from "./types";
import { getPointerValue } from "./utils/jsonPointer";
import { useResizableColumns } from "./hooks/useResizableColumns";
import { useSessionState } from "./hooks/useSessionState";
import { useSessionActions } from "./hooks/useSessionActions";

export default function App() {
  const { state, actions, dirtyRef } = useSessionState();
  const { sizes, startDrag } = useResizableColumns({ nav: 280, preview: 380 });

  const {
    initializeSession,
    handleChange,
    handleSave,
    handleExit,
    handlePreviewFormatChange,
    handlePreviewPrettyChange,
  } = useSessionActions({ state, actions, dirtyRef });

  // Initialize session on mount (empty deps to run only once)
  useEffect(() => {
    initializeSession();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selectedNode = useMemo(
    () =>
      findNodeByPointer(
        state.session?.ui_ast?.roots ?? [],
        state.selectedPointer,
      ),
    [state.session, state.selectedPointer],
  );

  // Destructure state for easier access
  const {
    session,
    data,
    loading,
    dirty,
    saving,
    exiting,
    sessionEnded,
    selectedPointer,
    errors,
    formats,
    previewFormat,
    previewPretty,
    previewPayload,
    showErrorsDialog,
    status,
  } = state;

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center bg-background text-foreground">
        <div className="text-center space-y-2">
          <div className="h-8 w-8 mx-auto animate-spin rounded-full border-2 border-primary border-t-transparent" />
          <p className="text-sm text-muted-foreground">Loading session…</p>
        </div>
      </div>
    );
  }

  if (sessionEnded) {
    return (
      <div className="flex h-screen items-center justify-center bg-background text-foreground">
        <div className="text-center space-y-4">
          <div className="text-4xl">✓</div>
          <h1 className="text-xl font-semibold">Session Ended</h1>
          <p className="text-sm text-muted-foreground">
            You can close this browser tab.
          </p>
        </div>
      </div>
    );
  }

  return (
    <OverlayProvider>
      <div className="app-shell flex h-screen flex-col">
        <AppHeader
          title={session?.title ?? session?.ui_ast?.roots[0]?.title}
          description={session?.ui_ast?.roots[0]?.description}
          dirty={dirty}
          saving={saving}
          exiting={exiting}
          onSave={handleSave}
          onExit={() => handleExit()}
        />
        <div className="app-panel-muted flex flex-1 overflow-hidden border-y border-theme">
          {/* Navigation Panel */}
          <aside
            className="hidden md:flex app-panel flex-col border-r border-theme"
            style={{ width: sizes.nav }}
          >
            <div className="border-b border-theme px-4 py-3 text-xs uppercase tracking-[0.3em] text-muted-foreground">
              Schema TreeView
            </div>
            <TreeView
              ast={session?.ui_ast}
              selectedPointer={selectedPointer}
              errors={errors}
              onSelect={actions.setSelectedPointer}
            />
          </aside>
          {/* Resizer */}
          <div
            className="hidden md:block w-1 cursor-col-resize bg-transparent"
            onPointerDown={(event) => startDrag(event, "nav")}
          />
          {/* Main Editor Panel */}
          <main className="app-panel flex flex-1 flex-col overflow-hidden px-4 md:px-6 py-4">
            <EditorBreadcrumbs node={selectedNode} pointer={selectedPointer} />
            <div className="mt-4 flex-1 overflow-y-auto pr-2 md:pr-4 text-sm">
              {selectedNode
                ? (
                  <EditorBody
                    node={selectedNode}
                    data={data}
                    errors={errors}
                    onChange={handleChange}
                  />
                )
                : (
                  <div className="text-center text-muted-foreground">
                    Select a node from the tree to start editing.
                  </div>
                )}
            </div>
          </main>
          {/* Resizer */}
          <div
            className="hidden lg:block w-1 cursor-col-resize bg-transparent"
            onPointerDown={(event) => startDrag(event, "preview")}
          />
          {/* Preview Panel */}
          <section
            className="hidden lg:flex app-panel h-full flex-col border-l border-theme"
            style={{ width: sizes.preview }}
          >
            <PreviewPane
              formats={formats}
              format={previewFormat}
              onFormatChange={handlePreviewFormatChange}
              pretty={previewPretty}
              onPrettyChange={handlePreviewPrettyChange}
              payload={previewPayload}
              loading={false}
            />
          </section>
        </div>
        <StatusBar
          status={status}
          dirty={dirty}
          validating={false}
          errorCount={errors.size}
          onErrorsClick={errors.size > 0
            ? () => actions.setShowErrorsDialog(true)
            : undefined}
        />
        <ValidationErrorsDialog
          open={showErrorsDialog}
          onOpenChange={actions.setShowErrorsDialog}
          errors={errors}
          onNavigateToError={actions.setSelectedPointer}
        />
      </div>
    </OverlayProvider>
  );
}

function findNodeByPointer(
  nodes: UiNode[],
  pointer?: string,
): UiNode | undefined {
  if (!pointer) return nodes[0];
  for (const node of nodes) {
    if (node.pointer === pointer) {
      return node;
    }
    if (node.kind.type === "object") {
      const child = findNodeByPointer(node.kind.children ?? [], pointer);
      if (child) return child;
    }
  }
  return undefined;
}

function EditorBreadcrumbs(
  { node, pointer }: { node?: UiNode; pointer?: string },
) {
  const segments = pointerSegments(pointer);
  if (!segments.length) {
    return null;
  }
  return (
    <nav className="flex flex-wrap items-center gap-2 text-xs text-slate-600">
      {segments.map((segment, index) => (
        <span
          key={`${segment}-${index}`}
          className="rounded-full bg-white/5 px-3 py-1 text-slate-400"
        >
          {segment}
        </span>
      ))}
      {node?.required
        ? (
          <span className="text-[10px] uppercase tracking-[0.3em] text-rose-600">
            Required
          </span>
        )
        : null}
    </nav>
  );
}

function EditorBody({
  node,
  data,
  errors,
  onChange,
}: {
  node: UiNode;
  data: JsonValue;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
}) {
  if (node.kind.type === "object") {
    return (
      <div className="space-y-4">
        {(node.kind.children ?? []).map((child) => (
          <NodeRenderer
            key={child.pointer}
            node={child}
            value={getPointerValue(data, child.pointer)}
            errors={errors}
            onChange={onChange}
          />
        ))}
      </div>
    );
  }
  return (
    <NodeRenderer
      node={node}
      value={getPointerValue(data, node.pointer)}
      errors={errors}
      onChange={onChange}
    />
  );
}

function pointerSegments(pointer?: string) {
  if (!pointer || pointer === "/") return [];
  return pointer
    .split("/")
    .filter(Boolean)
    .map((segment) => segment.replace(/~1/g, "/").replace(/~0/g, "~"));
}
