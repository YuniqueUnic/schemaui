import { useEffect, useMemo, useState } from "react";
import { AppHeader } from "./components/AppHeader";
import { NodeRenderer } from "./components/NodeRenderer";
import { PreviewPane } from "./components/PreviewPane";
import { StatusBar } from "./components/StatusBar";
import { TreeView } from "./components/TreeView";
import { LayoutExplorer } from "./components/LayoutExplorer";
import { OverlayProvider } from "./components/Overlay";
import { ValidationErrorsDialog } from "./components/ValidationErrorsDialog";
import { Panel, PanelHeader } from "./components/Panel";
import { SegmentedControl } from "./components/SegmentedControl";
import type { JsonValue, UiNode } from "./types";
import { getPointerValue } from "./utils/jsonPointer";
import {
  findNodeByPointer,
  resolveNavigablePointer,
} from "./utils/nodeLookup";
import { useResizableColumns } from "./hooks/useResizableColumns";
import { useSessionState } from "./hooks/useSessionState";
import { useSessionActions } from "./hooks/useSessionActions";
import { useMediaQuery } from "./hooks/useMediaQuery";
import { cn } from "@/lib/utils";
import { FileText, ListTree } from "lucide-react";

type PanelView = "nav" | "editor" | "preview";

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
  const isDesktop = useMediaQuery("(min-width: 1024px)");
  const [mobileView, setMobileView] = useState<PanelView>("editor");

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
        <div className="app-panel-muted flex flex-1 flex-col overflow-hidden border-y border-theme lg:flex-row">
          {!isDesktop && (
            <MobilePanelSwitch
              value={mobileView}
              onChange={setMobileView}
            />
          )}
          {/* Navigation Panel */}
          <Panel
            as="aside"
            className={cn(
              "lg:flex lg:border-r border-theme",
              !isDesktop && mobileView === "nav" && "flex flex-1 border-b",
              !isDesktop && mobileView !== "nav" && "hidden",
            )}
            style={isDesktop ? { width: sizes.nav } : undefined}
          >
            <PanelHeader
              icon={<ListTree className="h-3.5 w-3.5" />}
              label="Navigation"
              actions={hasLayout
                ? (
                  <SegmentedControl<"schema" | "layout">
                    value={navMode}
                    onChange={(v) => setNavMode(v)}
                    options={[
                      { id: "schema", label: "Schema" },
                      { id: "layout", label: "Layout" },
                    ]}
                  />
                )
                : undefined}
            />
            {(!hasLayout || navMode === "schema") && (
              <TreeView
                ast={session?.ui_ast}
                selectedPointer={selectedPointer}
                errors={errors}
                onSelect={(pointer) => {
                  actions.setSelectedPointer(pointer);
                  if (!isDesktop) setMobileView("editor");
                }}
              />
            )}
            {hasLayout && navMode === "layout" && (
              <div className="flex-1 min-h-0 px-1 pb-2">
                <LayoutExplorer
                  layout={session.layout}
                  ast={session?.ui_ast}
                  selectedPointer={selectedPointer}
                  rootLabel={virtualRootTitle}
                  onSelect={(pointer) => {
                    actions.setSelectedPointer(pointer);
                    if (!isDesktop) setMobileView("editor");
                  }}
                />
              </div>
            )}
          </Panel>
          {/* Resizer */}
          <div
            className="app-resizer hidden lg:block"
            onPointerDown={(event) => startDrag(event, "nav")}
          />
          {/* Main Editor Panel */}
          <Panel
            as="main"
            className={cn(
              "lg:flex lg:flex-1",
              !isDesktop && mobileView === "editor" && "flex flex-1",
              !isDesktop && mobileView !== "editor" && "hidden",
            )}
          >
            <div className="flex flex-1 flex-col overflow-hidden px-4 py-4 md:px-6">
              <LayoutSectionNav
                roots={roots}
                rootLabel={virtualRootTitle}
                selectedPointer={selectedPointer}
                onSelect={actions.setSelectedPointer}
              />
              <EditorBreadcrumbs
                node={selectedNode}
                pointer={selectedPointer}
              />
              <div
                className="mt-4 flex-1 min-h-0 overflow-y-auto text-sm [scrollbar-gutter:stable_both-edges]"
              >
                <div className="px-1 py-1 pr-3 pb-8">
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
                      <div className="flex h-full items-center justify-center py-16">
                        <div className="text-center text-muted-foreground">
                          <FileText className="mx-auto mb-2 h-8 w-8 opacity-40" />
                          <p className="text-sm">
                            Select a node to start editing
                          </p>
                        </div>
                      </div>
                    )}
                </div>
              </div>
            </div>
          </Panel>
          {/* Resizer */}
          <div
            className="app-resizer hidden lg:block"
            onPointerDown={(event) => startDrag(event, "preview")}
          />
          {/* Preview Panel */}
          <Panel
            as="section"
            className={cn(
              "lg:flex lg:h-full lg:border-l border-theme",
              !isDesktop && mobileView === "preview" && "flex flex-1 border-t",
              !isDesktop && mobileView !== "preview" && "hidden",
            )}
            style={isDesktop ? { width: sizes.preview } : undefined}
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
          </Panel>
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
    <nav className="mb-1 flex flex-wrap items-center gap-1 text-xs text-muted-foreground">
      {segments.map((segment, index) => (
        <span key={`${segment}-${index}`} className="flex items-center gap-1">
          {index > 0 && <span className="opacity-40">/</span>}
          <span className="rounded-md bg-muted/60 px-2 py-0.5 font-mono text-[11px] text-foreground/80">
            {segment}
          </span>
        </span>
      ))}
      {node?.required && (
        <span className="ml-2 rounded-full bg-destructive/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.18em] text-destructive">
          Required
        </span>
      )}
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
      <div className="space-y-3">
        <EditorSectionIntro node={node} />
        {(node.kind.children ?? []).map((child) => (
          <div
            key={child.pointer}
            className="rounded-xl border border-theme bg-card/60 px-4 py-3 shadow-[0_1px_2px_rgba(0,0,0,0.04)] transition-colors hover:border-theme-strong"
          >
            <NodeRenderer
              node={child}
              value={getPointerValue(data, child.pointer)}
              errors={errors}
              onChange={onChange}
            />
          </div>
        ))}
      </div>
    );
  }
  return (
    <div className="rounded-xl border border-theme bg-card/60 px-4 py-3 shadow-[0_1px_2px_rgba(0,0,0,0.04)]">
      <NodeRenderer
        node={node}
        value={getPointerValue(data, node.pointer)}
        errors={errors}
        onChange={onChange}
      />
    </div>
  );
}

function EditorSectionIntro({ node }: { node: UiNode }) {
  const title = node.title?.trim();
  const description = node.description?.trim();
  const showHeader = node.pointer.length > 0 || !!description || node.required;

  if (!showHeader || (!title && !description && !node.required)) {
    return null;
  }

  return (
    <header className="space-y-1 px-1">
      <div className="flex items-center gap-2">
        <h2 className="text-sm font-semibold text-foreground">
          {title || node.pointer}
        </h2>
        {node.required && (
          <span className="rounded-full bg-destructive/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.18em] text-destructive">
            Required
          </span>
        )}
      </div>
      {description && (
        <p className="text-sm leading-relaxed text-muted-foreground">
          {description}
        </p>
      )}
    </header>
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
    <div className="mb-4 space-y-3 text-xs">
      {rows.map((row: TopbarRow, idx: number) => (
        <div
          key={`${row.containerPointer}|${idx}`}
          className={cn(
            "flex items-center gap-1.5 overflow-x-auto whitespace-nowrap rounded-xl px-1.5 py-0.5",
            "scrollbar-thin [scrollbar-width:thin] [&::-webkit-scrollbar]:h-0.5",
            idx === 0 ? "bg-muted/30" : "bg-transparent",
          )}
        >
          <button
            type="button"
            onClick={() => onSelect(row.containerPointer)}
            className={cn("shrink-0", navPillClass(
              row.activePointer === row.containerPointer,
              true,
            ))}
          >
            {row.containerTitle}
          </button>
          {row.children.length > 0 && (
            <span className="shrink-0 text-muted-foreground/50">/</span>
          )}
          {row.children.map((child: { pointer: string; title: string }) => (
            <button
              key={child.pointer}
              type="button"
              onClick={() => onSelect(child.pointer)}
              className={cn("shrink-0", navPillClass(
                row.activePointer === child.pointer,
                false,
              ))}
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
    return "app-pill app-pill-active font-medium";
  }
  if (isContainer) {
    return "app-pill app-pill-muted font-medium";
  }
  return "app-pill text-muted-foreground hover:bg-muted hover:text-foreground";
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

interface MobilePanelSwitchProps {
  value: PanelView;
  onChange(value: PanelView): void;
}

function MobilePanelSwitch({ value, onChange }: MobilePanelSwitchProps) {
  return (
    <div className="flex items-center justify-center border-b border-theme bg-background/80 px-2 py-2 backdrop-blur lg:hidden">
      <SegmentedControl<PanelView>
        value={value}
        onChange={onChange}
        size="md"
        options={[
          { id: "nav", label: "Nav" },
          { id: "editor", label: "Editor" },
          { id: "preview", label: "Preview" },
        ]}
      />
    </div>
  );
}
