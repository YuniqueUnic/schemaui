import { useEffect, useMemo, useState } from "react";
import { AppHeader } from "./components/AppHeader";
import { NodeRenderer } from "./components/NodeRenderer";
import { PreviewPane } from "./components/PreviewPane";
import { StatusBar } from "./components/StatusBar";
import { TreeView } from "./components/TreeView";
import { LayoutExplorer } from "./components/LayoutExplorer";
import { OverlayProvider } from "./components/Overlay";
import { ValidationErrorsDialog } from "./components/ValidationErrorsDialog";
import type { JsonValue, UiNode } from "./types";
import { getPointerValue } from "./utils/jsonPointer";
import {
  findNodeByPointer,
  resolveNavigablePointer,
} from "./utils/nodeLookup";
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

  const [navMode, setNavMode] = useState<"schema" | "layout">("schema");

  // Initialize session on mount (empty deps to run only once)
  useEffect(() => {
    initializeSession();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  const roots = session?.ui_ast?.roots ?? [];

  const virtualRootTitle = session?.layout?.roots?.[0]?.title ?? "General";

  const selectedNode = useMemo<UiNode | undefined>(() => {
    if (!state.selectedPointer) {
      if (roots.length === 0) return undefined;
      return {
        pointer: "",
        title: virtualRootTitle,
        description: null,
        required: false,
        default_value: null,
        kind: { type: "object", children: roots, required: [] },
      };
    }
    return findNodeByPointer(roots, state.selectedPointer);
  }, [roots, state.selectedPointer, virtualRootTitle]);

  const hasLayout = !!(session?.layout && session.layout.roots.length > 0);
  const focusLabel = selectedNode
    ? (selectedNode.title?.trim() || selectedNode.pointer)
    : (selectedPointer || undefined);

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
          title={session?.title}
          description={session?.description}
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
            <div className="border-b border-theme px-4 py-3 text-xs uppercase tracking-[0.3em] text-muted-foreground flex items-center justify-between gap-2">
              <span>Navigation</span>
              {hasLayout && (
                <div className="inline-flex rounded-full bg-muted p-0.5 text-[10px]">
                  <button
                    type="button"
                    onClick={() => setNavMode("schema")}
                    className={`px-2 py-0.5 rounded-full transition-colors ${
                      navMode === "schema"
                        ? "bg-background text-foreground"
                        : "text-muted-foreground"
                    }`}
                  >
                    Schema
                  </button>
                  <button
                    type="button"
                    onClick={() => setNavMode("layout")}
                    className={`px-2 py-0.5 rounded-full transition-colors ${
                      navMode === "layout"
                        ? "bg-background text-foreground"
                        : "text-muted-foreground"
                    }`}
                  >
                    Layout
                  </button>
                </div>
              )}
            </div>
            {(!hasLayout || navMode === "schema") && (
              <TreeView
                ast={session?.ui_ast}
                selectedPointer={selectedPointer}
                errors={errors}
                onSelect={actions.setSelectedPointer}
              />
            )}
            {hasLayout && navMode === "layout" && (
              <div className="flex-1 min-h-0 px-1 pb-2">
                <LayoutExplorer
                  layout={session.layout}
                  ast={session?.ui_ast}
                  selectedPointer={selectedPointer}
                  onSelect={actions.setSelectedPointer}
                />
              </div>
            )}
          </aside>
          {/* Resizer */}
          <div
            className="hidden lg:block w-1 cursor-col-resize bg-gray-200/60 relative hover:bg-gray-400/40"
            onPointerDown={(event) => startDrag(event, "nav")}
          >
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="w-px h-4 bg-gray-800"></div>
            </div>
          </div>
          {/* Main Editor Panel */}
          <main className="app-panel flex flex-1 flex-col overflow-hidden px-4 md:px-6 py-4">
            <LayoutSectionNav
              roots={roots}
              rootLabel={virtualRootTitle}
              selectedPointer={selectedPointer}
              onSelect={actions.setSelectedPointer}
            />
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
            className="hidden lg:block w-1 cursor-col-resize bg-gray-200/60 relative hover:bg-gray-400/40"
            onPointerDown={(event) => startDrag(event, "preview")}
          >
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="w-px h-4 bg-gray-800"></div>
            </div>
          </div>
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
          saving={saving}
          exiting={exiting}
          errorCount={errors.size}
          focusLabel={focusLabel}
          onErrorsClick={errors.size > 0
            ? () => actions.setShowErrorsDialog(true)
            : undefined}
        />
        <ValidationErrorsDialog
          open={showErrorsDialog}
          onOpenChange={actions.setShowErrorsDialog}
          errors={errors}
          onNavigateToError={(pointer) =>
            actions.setSelectedPointer(resolveNavigablePointer(roots, pointer))}
        />
      </div>
    </OverlayProvider>
  );
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

interface LayoutSectionNavProps {
  roots: UiNode[];
  rootLabel: string;
  selectedPointer?: string;
  onSelect(pointer: string): void;
}

interface TopbarRow {
  containerPointer: string;
  containerTitle: string;
  children: Array<{ pointer: string; title: string }>;
  activePointer: string;
}

function LayoutSectionNav(
  { roots, rootLabel, selectedPointer, onSelect }: LayoutSectionNavProps,
) {
  const rows = useMemo<TopbarRow[]>(
    () => buildTopbarRows(roots, rootLabel, selectedPointer ?? ""),
    [roots, rootLabel, selectedPointer],
  );

  if (rows.length === 0) return null;

  return (
    <div className="mb-3 space-y-1.5 text-xs">
      {rows.map((row, idx) => (
        <div
          key={`${row.containerPointer}|${idx}`}
          className="flex flex-wrap items-center gap-1.5 text-muted-foreground"
        >
          <button
            type="button"
            onClick={() => onSelect(row.containerPointer)}
            className={navPillClass(
              row.activePointer === row.containerPointer,
              true,
            )}
          >
            {row.containerTitle}
          </button>
          {row.children.map((child) => (
            <button
              key={child.pointer}
              type="button"
              onClick={() => onSelect(child.pointer)}
              className={navPillClass(
                row.activePointer === child.pointer,
                false,
              )}
            >
              {child.title}
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}

function navPillClass(active: boolean, isContainer: boolean): string {
  if (active) {
    return "rounded-full border border-primary bg-primary/10 px-3 py-1 text-foreground transition-colors";
  }
  if (isContainer) {
    return "rounded-full border border-transparent bg-muted/60 px-3 py-1 text-muted-foreground transition-colors hover:bg-muted";
  }
  return "rounded-full px-3 py-1 text-muted-foreground transition-colors hover:bg-muted";
}

function buildTopbarRows(
  roots: UiNode[],
  rootLabel: string,
  selectedPointer: string,
): TopbarRow[] {
  if (roots.length === 0) return [];

  const rows: TopbarRow[] = [];
  let containerPointer = "";
  let containerTitle = rootLabel;
  let containerChildren: UiNode[] = roots;

  // Safety: bound iterations to tree depth
  for (let depth = 0; depth < 32; depth++) {
    const matching = containerChildren.find((child) =>
      child.pointer === selectedPointer ||
      (selectedPointer.length > 0 &&
        selectedPointer.startsWith(`${child.pointer}/`))
    );

    const activePointer = matching
      ? matching.pointer
      : selectedPointer === containerPointer
      ? containerPointer
      : "";

    rows.push({
      containerPointer,
      containerTitle,
      children: containerChildren.map((child) => ({
        pointer: child.pointer,
        title: nodeLabel(child),
      })),
      activePointer,
    });

    if (!matching) break;
    if (matching.kind.type !== "object") break;
    const nextChildren = matching.kind.children ?? [];
    if (nextChildren.length === 0) break;

    containerPointer = matching.pointer;
    containerTitle = nodeLabel(matching);
    containerChildren = nextChildren;

    if (matching.pointer === selectedPointer) {
      rows.push({
        containerPointer,
        containerTitle,
        children: nextChildren.map((child) => ({
          pointer: child.pointer,
          title: nodeLabel(child),
        })),
        activePointer: containerPointer,
      });
      break;
    }
  }

  return rows;
}

function nodeLabel(node: UiNode): string {
  const title = node.title?.trim();
  if (title) return title;
  const segment = lastPointerSegment(node.pointer);
  return segment ?? node.pointer ?? "(root)";
}

function lastPointerSegment(pointer: string): string | undefined {
  if (!pointer || pointer === "/") return undefined;
  const segments = pointer.split("/").filter(Boolean);
  return segments[segments.length - 1]?.replace(/~1/g, "/").replace(/~0/g, "~");
}
