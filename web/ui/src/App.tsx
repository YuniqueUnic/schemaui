import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { AppHeader } from "./components/AppHeader";
import { NodeRenderer, RequiredTag } from "./components/NodeRenderer";
import { PreviewPane } from "./components/PreviewPane";
import { StatusBar } from "./components/StatusBar";
import { TreeView } from "./components/TreeView";
import { LayoutExplorer } from "./components/LayoutExplorer";
import { OverlayProvider } from "./components/Overlay";
import { RichAssetsProvider } from "./components/rich/RichAssetsProvider";
import { HoverPreviewProvider } from "./components/rich/HoverPreviewLayer";
import { RichSurface } from "./components/rich/RichSurface";
import { EditorRuntimeProvider } from "./components/rich/EditorRuntime";
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
import { pruneHiddenNodes } from "./utils/visibility";
import { useSessionState } from "./hooks/useSessionState";
import { useSessionActions } from "./hooks/useSessionActions";
import { useMediaQuery } from "./hooks/useMediaQuery";
import { useCountdown } from "./hooks/useCountdown";
import { httpTransport } from "./transport/httpTransport";
import type { SchemaUiTransport } from "./transport/types";
import { cn } from "@/lib/utils";
import {
  ChevronLeft,
  ChevronRight,
  FileText,
  ListTree,
  TimerOff,
  TriangleAlert,
} from "lucide-react";

type PanelView = "nav" | "editor" | "preview";

/**
 * What the whole-document entry is called, in the tab band and the tree.
 *
 * A fixed word rather than the document's title: the title is a sentence
 * ("Service rollout") that reads as a heading, and a tab needs a label. It also
 * must not come from `layout.roots[0].title` — that is the *first section*, so
 * using it labelled the root tab with the name of one of its own children.
 */
const ROOT_LABEL = "General";

interface AppProps {
  /** Where the schema pipeline runs. Defaults to the HTTP session this app
   * was built for; the Playground passes a wasm-backed transport instead. */
  transport?: SchemaUiTransport;
  /** Playground only: returns to the paste screen with what was typed there
   * still in it. Absent for an HTTP session, which has nothing to go back to
   * — see `AppHeader`'s doc comment on the same prop. */
  onBack?: () => void;
}

export default function App({ transport = httpTransport, onBack }: AppProps = {}) {
  const { state, actions, dirtyRef } = useSessionState();
  const { sizes, startDrag, isDragging } = useResizableColumns({ nav: 280, preview: 380 });

  const {
    initializeSession,
    handleChange,
    handleSave,
    handleExit,
    handlePreviewFormatChange,
    handlePreviewPrettyChange,
  } = useSessionActions({ state, actions, dirtyRef, transport });

  const [navMode, setNavMode] = useState<"schema" | "layout">("schema");
  const isDesktop = useMediaQuery("(min-width: 1024px)");
  const [mobileView, setMobileView] = useState<PanelView>("editor");
  const [navCollapsed, setNavCollapsed] = useState(false);
  // Below ~1150px the editor is the column that loses when the preview is open:
  // measured against the widest label in the demo schema, the editor needs about
  // 440px before select labels start clipping (at 1024px it gets 356px and three
  // of five clip; at 1100px it gets 432px and none do). 1152 leaves a margin for
  // longer labels. Below that the preview starts folded — still one click away
  // in its own rail, and the user can widen it if they disagree.
  // Seeded once, deliberately not kept in sync: re-collapsing on every resize
  // would fight someone who just opened it.
  const [previewCollapsed, setPreviewCollapsed] = useState(
    () =>
      typeof window !== "undefined" &&
      !window.matchMedia("(min-width: 1152px)").matches,
  );

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
    previewError,
    showErrorsDialog,
    status,
  } = state;

  // Visibility is derived, not stored: pruning once here keeps the tree, the
  // layout explorer and the editor in agreement about what the form shows.
  const uiAst = useMemo(
    () => pruneHiddenNodes(session?.ui_ast, data),
    [session?.ui_ast, data],
  );

  const roots = useMemo(() => uiAst?.roots ?? [], [uiAst]);

  const selectedNode = useMemo<UiNode | undefined>(() => {
    if (!state.selectedPointer) {
      if (roots.length === 0) return undefined;
      return {
        pointer: "",
        title: ROOT_LABEL,
        description: null,
        required: false,
        default_value: null,
        kind: { type: "object", children: roots, required: [] },
      };
    }
    return findNodeByPointer(roots, state.selectedPointer);
  }, [roots, state.selectedPointer]);

  const hasLayout = !!(session?.layout && session.layout.roots.length > 0);
  const focusLabel = selectedNode
    ? (selectedNode.title?.trim() || selectedNode.pointer)
    : (selectedPointer || undefined);

  // Ticks against the duration the backend handed out. Reaching zero is a
  // terminal state: the server has already stopped answering, so there is
  // nothing left to save to and no point offering the form.
  const secondsLeft = useCountdown(session?.expires_in_ms);
  const timedOut = secondsLeft === 0;

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

  // Checked before the countdown: saving at the last second closes the session
  // legitimately, and that success must not be relabelled as a timeout.
  if (sessionEnded) {
    return (
      <SessionNotice
        icon={<span className="text-4xl">✓</span>}
        title="Session Ended"
        message="You can close this browser tab."
      />
    );
  }

  if (timedOut) {
    return (
      <SessionNotice
        icon={<TimerOff className="h-9 w-9 text-rose-500" aria-hidden="true" />}
        title="Session Timed Out"
        message="This session closed itself when its deadline passed, so nothing was saved. Ask whoever started it to run it again."
      />
    );
  }

  if (!session) {
    return (
      <SessionNotice
        icon={
          <TriangleAlert className="h-9 w-9 text-amber-500" aria-hidden="true" />
        }
        title="Session Unavailable"
        message="This session is no longer being served. It may have timed out or already been closed — ask whoever started it to run it again."
      />
    );
  }

  return (
    // RichAssets wraps Overlay: the fullscreen viewer renders through the
    // Overlay's own subtree, and context flows with the React tree, not the
    // DOM — a provider outside Overlay is invisible to the viewer.
    <RichAssetsProvider assets={session.rich}>
      <OverlayProvider>
        <EditorRuntimeProvider
          transport={transport}
          capabilities={session.capabilities}
        >
          <HoverPreviewProvider>
          <div className="app-shell flex h-screen flex-col">
            <AppHeader
              title={session?.title}
              description={session?.description}
              saving={saving}
              exiting={exiting}
              secondsLeft={secondsLeft}
              onSave={handleSave}
              onExit={() => handleExit()}
              exitLabel={transport.kind === "wasm" ? "Export" : "Exit"}
              exitingLabel={transport.kind === "wasm" ? "Exporting…" : "Exiting…"}
              onBack={onBack}
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
                  !isDragging && "transition-[width] duration-150",
                  !isDesktop && mobileView === "nav" && "flex flex-1 border-b",
                  !isDesktop && mobileView !== "nav" && "hidden",
                  isDesktop && navCollapsed && "!w-10 overflow-hidden",
                )}
                style={isDesktop && !navCollapsed ? { width: sizes.nav } : undefined}
              >
                <PanelHeader
                  icon={<ListTree className="h-3.5 w-3.5" />}
                  // No text label: this panel is the narrowest column, and
                  // "Navigation" was being clipped to "NAVIGA…" by the Schema/Layout
                  // switch sitting next to it. The icon and the tree below say the
                  // same thing without the collision.
                  actions={
                    <div className="flex items-center gap-1">
                      {(!isDesktop || !navCollapsed) && hasLayout && (
                        <SegmentedControl<"schema" | "layout">
                          value={navMode}
                          onChange={(v) => setNavMode(v)}
                          options={[
                            { id: "schema", label: "Schema" },
                            { id: "layout", label: "Layout" },
                          ]}
                        />
                      )}
                      {isDesktop && (
                        <button
                          type="button"
                          onClick={() => setNavCollapsed((v) => !v)}
                          aria-label={navCollapsed ? "Expand navigation" : "Collapse navigation"}
                          className="flex items-center justify-center rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
                        >
                          {navCollapsed
                            ? <ChevronRight className="h-3.5 w-3.5" />
                            : <ChevronLeft className="h-3.5 w-3.5" />}
                        </button>
                      )}
                    </div>
                  }
                />
                {/* Show content when: on mobile (always), or on desktop when not collapsed */}
                {(!isDesktop || !navCollapsed) && (
                  <>
                    {(!hasLayout || navMode === "schema") && (
                      <TreeView
                        ast={uiAst}
                        selectedPointer={selectedPointer}
                        errors={errors}
                        rootLabel={ROOT_LABEL}
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
                          ast={uiAst}
                          selectedPointer={selectedPointer}
                          rootLabel={ROOT_LABEL}
                          onSelect={(pointer) => {
                            actions.setSelectedPointer(pointer);
                            if (!isDesktop) setMobileView("editor");
                          }}
                        />
                      </div>
                    )}
                  </>
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
                <div className="flex flex-1 flex-col overflow-hidden">
                  <SectionTabs
                    roots={roots}
                    rootLabel={ROOT_LABEL}
                    selectedPointer={selectedPointer}
                    onSelect={actions.setSelectedPointer}
                  />
                  <div className="flex-1 min-h-0 overflow-y-auto [scrollbar-gutter:stable_both-edges]">
                    <div className="mx-auto w-full max-w-3xl px-4 py-5 md:px-6">
                      {selectedNode
                        ? (
                          <>
                            <EditorHeading
                              node={selectedNode}
                              isRoot={roots.some((root) =>
                                root.pointer === selectedNode.pointer
                              )}
                            />
                            {/* The virtual root has no fields of its own — it is
                                every section at once, so each one is rendered with
                                the heading the tabs would otherwise have carried. */}
                            {!selectedNode.pointer && roots.length > 0
                              ? (
                                <GeneralView
                                  roots={roots}
                                  data={data}
                                  errors={errors}
                                  onChange={handleChange}
                                />
                              )
                              : (
                                <EditorBody
                                  node={selectedNode}
                                  data={data}
                                  errors={errors}
                                  onChange={handleChange}
                                />
                              )}
                          </>
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
              {/* Preview Panel — outer Panel provided by PreviewPane internally */}
              <PreviewPaneWithToggle
                collapsed={previewCollapsed}
                onToggle={() => setPreviewCollapsed((v) => !v)}
                isDesktop={isDesktop}
                isDragging={isDragging}
                mobileVisible={!isDesktop && mobileView === "preview"}
                previewWidth={sizes.preview}
                formats={formats}
                format={previewFormat}
                onFormatChange={handlePreviewFormatChange}
                pretty={previewPretty}
                onPrettyChange={handlePreviewPrettyChange}
                payload={previewPayload}
                error={previewError}
              />
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
          </HoverPreviewProvider>
        </EditorRuntimeProvider>
      </OverlayProvider>
    </RichAssetsProvider>
  );
}

/**
 * Full-screen terminal state: the session is over and there is no form left to
 * show. Shared by "saved", "timed out" and "server gone" so the three read as
 * one family rather than three different apps.
 */
function SessionNotice({
  icon,
  title,
  message,
}: {
  icon: ReactNode;
  title: string;
  message: string;
}) {
  return (
    <div
      data-testid="session-notice"
      role="alert"
      className="flex h-screen items-center justify-center bg-background px-6 text-foreground"
    >
      <div className="max-w-md text-center space-y-4">
        <div className="flex justify-center">{icon}</div>
        <h1 className="text-xl font-semibold">{title}</h1>
        <p className="text-sm text-muted-foreground">{message}</p>
      </div>
    </div>
  );
}

/**
 * The editor's one navigation band: "General", then the top-level sections, as
 * underlined tabs.
 *
 * It used to be two rows of pill chips — one per level of the path — plus a
 * breadcrumb and a section heading, which named the same section up to four
 * times. One stable row is enough: it answers "which part of the form am I in",
 * and the tree on the left remains the full hierarchy.
 *
 * "General" is the whole document on one page, which is also where the session
 * opens. Without it there was no way back to that view once a section had been
 * picked, so browsing the form meant visiting every section in turn.
 */
function SectionTabs({
  roots,
  rootLabel,
  selectedPointer,
  onSelect,
}: {
  roots: UiNode[];
  rootLabel: string;
  selectedPointer?: string;
  onSelect(pointer: string): void;
}) {
  if (roots.length === 0) return null;

  const activePointer = roots.find(
    (root) =>
      selectedPointer === root.pointer ||
      selectedPointer?.startsWith(`${root.pointer}/`),
  )?.pointer;

  const tabs = [
    { pointer: "", label: rootLabel },
    ...roots.map((root) => ({ pointer: root.pointer, label: nodeLabel(root) })),
  ];

  // One section already is the whole document, so a band of tabs would offer
  // two names for the same page. The strip still exists and is still the same
  // height — a column whose header vanished would break the alignment the
  // other two share — but it carries the name instead of a control.
  const showTabs = roots.length >= 2;
  const stripClass =
    "app-panel-header shrink-0 border-b border-theme px-4 md:px-6";

  if (!showTabs) {
    return (
      <div className={stripClass}>
        <div className="mx-auto flex h-full w-full max-w-3xl items-center">
          <span className="text-xs font-medium text-foreground">
            {nodeLabel(roots[0])}
          </span>
        </div>
      </div>
    );
  }

  return (
    <nav aria-label="Sections" className={stripClass}>
      {/* `items-stretch` plus a full-height button puts each tab's underline on
          the strip's bottom edge, which is what makes the band read as one
          row rather than as buttons floating inside a box. */}
      <div className="mx-auto flex h-full w-full max-w-3xl items-stretch gap-1 overflow-x-auto">
        {tabs.map((tab) => {
          const active = tab.pointer === ""
            ? !selectedPointer
            : tab.pointer === activePointer;
          return (
            <button
              key={tab.pointer || "__general"}
              type="button"
              onClick={() => onSelect(tab.pointer)}
              aria-current={active ? "true" : undefined}
              className={cn(
                "flex shrink-0 items-center whitespace-nowrap border-b-2 px-2.5 text-xs transition-colors",
                active
                  ? "border-primary font-medium text-foreground"
                  : "border-transparent text-muted-foreground hover:border-theme-strong hover:text-foreground",
              )}
            >
              {tab.label}
            </button>
          );
        })}
      </div>
    </nav>
  );
}

/**
 * The heading for whatever is selected: title, required marker, description.
 *
 * A root section is already named by the tab above it, so its title is
 * suppressed here — printing the same words one line apart was the last of the
 * four places this screen used to name the same thing. Its description stays,
 * because that is the part the tab cannot carry.
 *
 * A titleless node falls back to its pointer, rendered in mono so it reads as a
 * machine name rather than a title someone wrote.
 */
function EditorHeading({ node, isRoot }: { node: UiNode; isRoot: boolean }) {
  const title = node.title?.trim();
  const description = node.description?.trim();

  // The virtual root has no pointer of its own, so its heading would only
  // restate the session title in the header above; it earns one only when it
  // has something of its own to say.
  if (!node.pointer && !description) return null;

  const showTitle = !isRoot && Boolean(title || node.pointer);
  if (!showTitle && !description) return null;

  return (
    <header className="mb-4">
      {showTitle && (
        <div className="flex items-center gap-2">
          <h2
            className={cn(
              "text-[15px] font-semibold tracking-tight",
              title ? "text-foreground" : "font-mono text-sm text-muted-foreground",
            )}
          >
            {title || node.pointer}
          </h2>
          {node.required && <RequiredTag />}
        </div>
      )}
      {description && (
        <p
          className={cn(
            "max-w-[68ch] text-xs leading-relaxed text-muted-foreground",
            showTitle && "mt-1",
          )}
        >
          {description}
        </p>
      )}
    </header>
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
  // One card style for every field. A solid surface on the muted panel is what
  // makes the divisions legible; the old translucent fill plus drop shadow made
  // the cards read as floating chips rather than a list.
  const card =
    "rounded-lg border border-theme bg-card px-4 py-3.5 transition-colors hover:border-theme-strong";

  if (node.kind.type === "object") {
    return (
      <div className="space-y-2.5">
        {(node.kind.children ?? []).map((child) => (
          <div key={child.pointer} className={card}>
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
    <div className={card}>
      <NodeRenderer
        node={node}
        value={getPointerValue(data, node.pointer)}
        errors={errors}
        onChange={onChange}
        hideHeader
      />
    </div>
  );
}

/**
 * Every section on one page, in document order.
 *
 * This is what the "General" tab and the tree's root row show. Each section
 * keeps its own heading and its own card list, so the page still reads as
 * sections rather than as one undifferentiated column of fields — the point is
 * to remove the need to *navigate*, not to remove the structure.
 *
 * Rendered through the same `EditorBody` the single-section view uses, so the
 * two cannot drift apart in spacing or card treatment.
 */
function GeneralView({
  roots,
  data,
  errors,
  onChange,
}: {
  roots: UiNode[];
  data: JsonValue;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
}) {
  return (
    <div className="space-y-8">
      {roots.map((root) => (
        <section key={root.pointer} aria-labelledby={`section-${root.pointer}`}>
          <h2
            id={`section-${root.pointer}`}
            className="mb-2 flex items-center gap-2 text-[13px] font-semibold tracking-tight text-foreground"
          >
            {nodeLabel(root)}
            {root.required && <RequiredTag />}
          </h2>
          {root.description && (
            <p className="mb-2.5 max-w-[68ch] text-xs leading-relaxed text-muted-foreground">
              {root.description}
            </p>
          )}
          {root.content && (
            <div className="mb-3">
              <RichSurface
                content={root.content}
                variant="block"
                title={nodeLabel(root)}
              />
            </div>
          )}
          <EditorBody
            node={root}
            data={data}
            errors={errors}
            onChange={onChange}
          />
        </section>
      ))}
    </div>
  );
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
    <div className="app-panel-header justify-center border-b border-theme bg-background/80 px-2 backdrop-blur lg:hidden">
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

interface PreviewPaneWithToggleProps {
  collapsed: boolean;
  onToggle: () => void;
  isDesktop: boolean;
  isDragging: boolean;
  mobileVisible: boolean;
  previewWidth: number;
  formats: string[];
  format: string;
  onFormatChange: (value: string) => void;
  pretty: boolean;
  onPrettyChange: (value: boolean) => void;
  payload: string;
  error?: string | null;
}

function PreviewPaneWithToggle({
  collapsed,
  onToggle,
  isDesktop,
  isDragging,
  mobileVisible,
  previewWidth,
  ...previewProps
}: PreviewPaneWithToggleProps) {
  if (!isDesktop) {
    // On mobile: show/hide via className, no collapse toggle
    if (!mobileVisible) return null;
    return (
      <section className="flex flex-1 flex-col overflow-hidden border-t border-theme">
        <PreviewPane {...previewProps} loading={false} />
      </section>
    );
  }

  return (
    <section
      className={cn(
        "lg:flex lg:h-full lg:border-l border-theme flex flex-col overflow-hidden",
        !isDragging && "transition-[width] duration-150",
        collapsed && "!w-10",
      )}
      style={!collapsed ? { width: previewWidth } : undefined}
    >
      {collapsed
        ? (
          <div className="flex h-full flex-col items-center pt-3">
            <button
              type="button"
              onClick={onToggle}
              aria-label="Expand preview"
              className="flex flex-col items-center gap-1.5 rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
            >
              <ChevronLeft className="h-3.5 w-3.5" />
            </button>
          </div>
        )
        : <PreviewPane {...previewProps} loading={false} onToggleCollapse={onToggle} />}
    </section>
  );
}
