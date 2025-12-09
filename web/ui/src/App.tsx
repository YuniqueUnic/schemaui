import { useEffect, useMemo, useState } from "react";
import { AppHeader } from "./components/AppHeader";
import { NodeRenderer } from "./components/NodeRenderer";
import { PreviewPane } from "./components/PreviewPane";
import { StatusBar } from "./components/StatusBar";
import { TreeView } from "./components/TreeView";
import { LayoutExplorer } from "./components/LayoutExplorer";
import { OverlayProvider } from "./components/Overlay";
import { ValidationErrorsDialog } from "./components/ValidationErrorsDialog";
import type { JsonValue, UiLayout, UiNode } from "./types";
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

  const [navMode, setNavMode] = useState<"schema" | "layout">("schema");

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

  const hasLayout = !!(session?.layout && session.layout.roots.length > 0);

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
              layout={session?.layout}
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

interface LayoutSectionNavProps {
  layout?: UiLayout | null;
  selectedPointer?: string;
  onSelect(pointer: string): void;
}

interface NavRoot {
  id: string;
  title: string;
  sections: NavSection[];
}

interface NavSection {
  id: string;
  title: string;
  firstPointer?: string;
  pointers: string[];
}

function LayoutSectionNav(
  { layout, selectedPointer, onSelect }: LayoutSectionNavProps,
) {
  const model = useMemo<NavRoot[]>(
    () => layout && layout.roots.length > 0 ? buildNavModel(layout) : [],
    [layout],
  );

  if (!model.length) return null;

  let activeRoot = model[0];
  let activeSection = activeRoot.sections[0] ?? null;

  if (selectedPointer) {
    outer: for (const root of model) {
      for (const section of root.sections) {
        if (section.pointers.includes(selectedPointer)) {
          activeRoot = root;
          activeSection = section;
          break outer;
        }
      }
    }
  }

  const handleRootClick = (root: NavRoot) => {
    const target = root.sections[0];
    if (target?.firstPointer) {
      onSelect(target.firstPointer);
    }
  };

  const handleSectionClick = (section: NavSection) => {
    if (section.firstPointer) {
      onSelect(section.firstPointer);
    }
  };

  return (
    <div className="mb-3 space-y-2 text-xs">
      <div className="flex flex-wrap items-center gap-2 text-muted-foreground">
        {model.map((root) => {
          const isActive = root.id === activeRoot.id;
          return (
            <button
              key={root.id}
              type="button"
              onClick={() => handleRootClick(root)}
              className={`rounded-full border px-3 py-1 transition-colors ${
                isActive
                  ? "border-primary bg-primary/10 text-foreground"
                  : "border-transparent bg-muted text-muted-foreground hover:bg-muted/80"
              }`}
            >
              {root.title || "Root"}
            </button>
          );
        })}
      </div>
      {activeRoot.sections.length > 1 && (
        <div className="flex flex-wrap gap-1">
          {activeRoot.sections.map((section) => {
            const isActive = !!activeSection && section.id === activeSection.id;
            return (
              <button
                key={section.id}
                type="button"
                onClick={() => handleSectionClick(section)}
                className={`rounded-full px-3 py-1 transition-colors ${
                  isActive
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted text-muted-foreground hover:bg-muted/80"
                }`}
              >
                {section.title}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

type LayoutSectionType = UiLayout["roots"][number]["sections"][number];

function buildNavModel(layout: UiLayout): NavRoot[] {
  const roots: NavRoot[] = [];

  for (const root of layout.roots) {
    const sections: NavSection[] = [];
    for (const section of root.sections) {
      collectSection(section, sections);
    }
    if (!sections.length) continue;
    roots.push({
      id: root.id,
      title: root.title ?? "Root",
      sections,
    });
  }

  return roots;
}

function collectSection(section: LayoutSectionType, out: NavSection[]) {
  const pointers: string[] = [];
  collectPointers(section, pointers);
  const firstPointer = findFirstPointer(section);
  out.push({
    id: section.id,
    title: section.title,
    firstPointer,
    pointers,
  });

  for (const child of section.children) {
    collectSection(child, out);
  }
}

function collectPointers(section: LayoutSectionType, out: string[]) {
  for (const fp of section.field_pointers) {
    out.push(fp);
  }
  if (section.pointer) {
    out.push(section.pointer);
  }
  for (const child of section.children) {
    collectPointers(child, out);
  }
}

function findFirstPointer(section: LayoutSectionType): string | undefined {
  if (section.field_pointers.length > 0) {
    return section.field_pointers[0];
  }
  for (const child of section.children) {
    const childPtr = findFirstPointer(child);
    if (childPtr) return childPtr;
  }
  return section.pointer || undefined;
}
