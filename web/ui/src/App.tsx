import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { clsx } from "clsx";
import { AppHeader } from "./components/AppHeader";
import { TreePanel } from "./components/TreePanel";
import { EditorPane } from "./components/EditorPane";
import { PreviewPane } from "./components/PreviewPane";
import { StatusBar } from "./components/StatusBar";
import { ExitGuardDialog } from "./components/ExitGuardDialog";
import { useResizableColumns } from "./hooks/useResizableColumns";
import { applyBlueprintDefaults } from "./utils/defaults";
import {
  buildSectionTree,
  getBreadcrumbs,
  getSectionByPath,
  type SectionPath,
  type TreeNode,
} from "./utils/blueprint";
import {
  exitSession,
  fetchSession,
  persistData,
  renderPreview,
  validateData,
} from "./api";
import type { JsonValue, ValidationResponse, WebBlueprint } from "./types";
import { deepClone, setPointerValue } from "./utils/jsonPointer";
import { useTheme } from "./theme";
import "./index.css";

interface ToastState {
  message: string;
  kind: "success" | "error";
}

export default function App() {
  const [blueprint, setBlueprint] = useState<WebBlueprint | undefined>();
  const [sessionTitle, setSessionTitle] = useState<string | null | undefined>();
  const [formats, setFormats] = useState<string[]>(["json"]);
  const [data, setData] = useState<JsonValue>({});
  const [status, setStatus] = useState("Loading schema…");
  const [dirty, setDirty] = useState(false);
  const [validating, setValidating] = useState(false);
  const [validationErrors, setValidationErrors] = useState<Map<string, string>>(
    () => new Map(),
  );
  const [previewFormat, setPreviewFormat] = useState("json");
  const [previewPretty, setPreviewPretty] = useState(true);
  const [previewPayload, setPreviewPayload] = useState("{}");
  const [activePath, setActivePath] = useState<SectionPath>({
    rootIndex: 0,
    sectionPath: [],
  });
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const [treeFilter, setTreeFilter] = useState("");
  const [lastSaved, setLastSaved] = useState<Date | null>(null);
  const [saving, setSaving] = useState(false);
  const [toast, setToast] = useState<ToastState | null>(null);
  const [sessionLoading, setSessionLoading] = useState(true);
  const [previewPending, setPreviewPending] = useState(false);
  const [exitPromptOpen, setExitPromptOpen] = useState(false);
  const [exitPending, setExitPending] = useState(false);
  const [exitCheckPending, setExitCheckPending] = useState(false);
  const validationSequence = useRef(0);
  const previewSequence = useRef(0);

  const { sizes, startDrag } = useResizableColumns({ nav: 280, preview: 420 });
  const { theme } = useTheme();
  const isMac = typeof navigator !== "undefined" &&
    /(Mac|iPhone|iPad)/i.test(
      navigator?.platform ?? navigator?.userAgent ?? "",
    );
  const modifierKey = isMac ? "⌘" : "Ctrl";

  useEffect(() => {
    let mounted = true;
    setSessionLoading(true);
    (async () => {
      try {
        const payload = await fetchSession();
        if (!mounted) return;
        const normalized = applyBlueprintDefaults(
          payload.blueprint,
          payload.data ?? {},
        );
        setBlueprint(payload.blueprint);
        setSessionTitle(payload.title ?? payload.blueprint?.title);
        const availableFormats =
          payload.formats?.length && payload.formats.length > 0
            ? payload.formats
            : ["json"];
        setFormats(availableFormats);
        setPreviewFormat(
          availableFormats.includes("json") ? "json" : availableFormats[0],
        );
        setData(normalized);
        setStatus("Ready");
        const initialPath = resolveInitialPath(payload.blueprint);
        setActivePath(initialPath);
        setExpanded(
          new Set(
            payload.blueprint.roots.map(
              (root, index) => root.id || root.title || `root-${index}`,
            ),
          ),
        );
        setDirty(false);
        setValidationErrors(new Map());
        await Promise.all([
          runValidation(normalized),
          updatePreview(
            normalized,
            availableFormats[0] || "json",
            previewPretty,
          ),
        ]);
      } catch (error) {
        console.error(error);
        setStatus("Failed to load schema payload");
        setToast({ message: "Unable to load schema", kind: "error" });
      } finally {
        if (mounted) {
          setSessionLoading(false);
        }
      }
    })();
    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        handleSave();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  const sectionTree = useMemo(
    () => buildSectionTree(blueprint),
    [blueprint],
  );

  const activeSection = useMemo(
    () => getSectionByPath(blueprint, activePath),
    [blueprint, activePath],
  );

  const breadcrumbs = useMemo(
    () => getBreadcrumbs(blueprint, activePath),
    [blueprint, activePath],
  );

  const nodeErrorCounts = useMemo(
    () => aggregateErrors(sectionTree, validationErrors),
    [sectionTree, validationErrors],
  );

  const errorCount = validationErrors.size;
  const exitErrors = useMemo(
    () => Array.from(validationErrors.entries()),
    [validationErrors],
  );
  const shortcuts = useMemo(
    () => [
      { combo: `${modifierKey}+S`, label: "Save" },
    ],
    [modifierKey],
  );
  const exiting = exitPending || exitCheckPending;

  const handleFieldChange = useCallback(
    (pointer: string, value: JsonValue) => {
      setDirty(true);
      setData((prev) => setPointerValue(prev ?? {}, pointer, deepClone(value)));
    },
    [],
  );

  const handleTreeSelect = useCallback((path: SectionPath, nodeId: string) => {
    setActivePath(path);
    setExpanded((prev) => new Set(prev).add(nodeId));
  }, []);

  const handleToggleNode = useCallback((nodeId: string) => {
    setExpanded((prev) => {
      const clone = new Set(prev);
      if (clone.has(nodeId)) {
        clone.delete(nodeId);
      } else {
        clone.add(nodeId);
      }
      return clone;
    });
  }, []);

  const applyValidationResult = useCallback(
    (result: ValidationResponse) => {
      const next = new Map<string, string>();
      result.errors.forEach((error) => {
        next.set(error.pointer || "/", error.message);
      });
      setValidationErrors(next);
      setStatus(result.ok ? "Validation passed" : "Validation failed");
      return result;
    },
    [],
  );

  const runValidation = useCallback(
    async (draft: JsonValue): Promise<ValidationResponse | undefined> => {
      const seq = validationSequence.current + 1;
      validationSequence.current = seq;
      setValidating(true);
      try {
        const result = await validateData(draft);
        if (seq === validationSequence.current) {
          applyValidationResult(result);
        }
        return result;
      } catch (error) {
        console.error(error);
        if (seq === validationSequence.current) {
          setStatus("Validation request failed");
          setToast({ message: "Validation failed", kind: "error" });
        }
      } finally {
        if (seq === validationSequence.current) {
          setValidating(false);
        }
      }
    },
    [applyValidationResult],
  );

  const updatePreview = useCallback(
    async (draft: JsonValue, format: string, pretty: boolean) => {
      const seq = previewSequence.current + 1;
      previewSequence.current = seq;
      setPreviewPending(true);
      try {
        const payload = await renderPreview(draft, format, pretty);
        if (seq !== previewSequence.current) {
          return;
        }
        setPreviewPayload(payload.payload);
      } catch (error) {
        console.error(error);
        if (seq === previewSequence.current) {
          setPreviewPayload("// Unable to render preview");
        }
      } finally {
        if (seq === previewSequence.current) {
          setPreviewPending(false);
        }
      }
    },
    [],
  );

  useEffect(() => {
    if (sessionLoading) {
      return;
    }
    const timer = window.setTimeout(() => {
      runValidation(data);
      updatePreview(data, previewFormat, previewPretty);
    }, 240);
    return () => window.clearTimeout(timer);
  }, [
    data,
    previewFormat,
    previewPretty,
    sessionLoading,
    runValidation,
    updatePreview,
  ]);

  const handleSave = useCallback(async () => {
    if (saving) return;
    setSaving(true);
    setStatus("Saving…");
    try {
      await persistData(data);
      setDirty(false);
      setLastSaved(new Date());
      setToast({ message: "Configuration saved", kind: "success" });
      setStatus("All changes saved.");
    } catch (error) {
      console.error(error);
      setToast({ message: "Save failed", kind: "error" });
      setStatus("Save failed");
    } finally {
      setSaving(false);
    }
  }, [data, saving]);

  const finalizeExit = useCallback(
    async (commit: boolean) => {
      if (exitPending) {
        return;
      }
      setExitPending(true);
      try {
        await exitSession(data, commit);
        setToast({
          message: commit
            ? "Session closed with the latest changes."
            : "Session closed using the last saved configuration.",
          kind: "success",
        });
        setStatus("Session closed");
        setExitPromptOpen(false);
      } catch (error) {
        console.error(error);
        setToast({ message: "Exit failed", kind: "error" });
      } finally {
        setExitPending(false);
      }
    },
    [data, exitPending],
  );

  const handleExit = useCallback(async () => {
    if (exitCheckPending || exitPending) {
      return;
    }
    setExitCheckPending(true);
    try {
      const result = await runValidation(data);
      if (!result) {
        return;
      }
      if (result.errors.length > 0) {
        setExitPromptOpen(true);
        return;
      }
      await finalizeExit(true);
    } finally {
      setExitCheckPending(false);
    }
  }, [data, exitCheckPending, exitPending, finalizeExit, runValidation]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 2400);
    return () => window.clearTimeout(timer);
  }, [toast]);

  return (
    <div
      className={clsx(
        "flex h-screen min-h-0 flex-col overflow-hidden",
        theme === "dark"
          ? "bg-gradient-to-b from-slate-950 via-slate-950 to-slate-900 text-white"
          : "bg-gradient-to-b from-slate-50 via-white to-slate-100 text-slate-900",
      )}
    >
      <AppHeader
        title={sessionTitle}
        description={blueprint?.description}
        dirty={dirty}
        saving={saving}
        exiting={exiting}
        onSave={handleSave}
        onExit={handleExit}
      />
      <main className="flex flex-1 min-h-0 overflow-hidden">
        <div
          className="shrink-0 border-r border-slate-800/80"
          style={{ width: `${sizes.nav}px` }}
        >
          <TreePanel
            nodes={sectionTree}
            expanded={expanded}
            activeId={activeSection?.id}
            onToggle={handleToggleNode}
            onSelect={handleTreeSelect}
            filter={treeFilter}
            onFilterChange={setTreeFilter}
            errorCounts={nodeErrorCounts}
            loading={sessionLoading}
          />
        </div>
        <div
          className="w-2 cursor-col-resize bg-transparent"
          onPointerDown={(event) => startDrag(event, "nav")}
        />
        <section className="flex-1 min-h-0 bg-white dark:bg-slate-900/20">
          <EditorPane
            section={activeSection}
            data={data}
            errors={validationErrors}
            onChange={handleFieldChange}
            breadcrumbs={breadcrumbs}
            loading={sessionLoading}
          />
        </section>
        <div
          className="w-2 cursor-col-resize bg-transparent"
          onPointerDown={(event) => startDrag(event, "preview")}
        />
        <div
          className="shrink-0 border-l border-slate-200 bg-white dark:border-slate-800/80 dark:bg-transparent"
          style={{ width: `${sizes.preview}px` }}
        >
          <PreviewPane
            formats={formats}
            format={previewFormat}
            onFormatChange={setPreviewFormat}
            pretty={previewPretty}
            onPrettyChange={setPreviewPretty}
            payload={previewPayload}
            loading={previewPending}
          />
        </div>
      </main>
      <StatusBar
        status={status}
        dirty={dirty}
        validating={validating}
        errorCount={errorCount}
        lastSaved={lastSaved}
        shortcuts={shortcuts}
      />
      {exitPromptOpen
        ? (
          <ExitGuardDialog
            errors={exitErrors}
            forcing={exitPending}
            onCancel={() => setExitPromptOpen(false)}
            onForceExit={() => finalizeExit(false)}
          />
        )
        : null}
      {toast
        ? (
          <div
            className="pointer-events-none fixed bottom-8 left-1/2 z-50 -translate-x-1/2 rounded-full border border-slate-700/70 bg-slate-900/90 px-6 py-3 text-sm text-white shadow-xl"
            role="status"
          >
            {toast.message}
          </div>
        )
        : null}
    </div>
  );
}

function resolveInitialPath(blueprint: WebBlueprint): SectionPath {
  if (!blueprint.roots.length) {
    return { rootIndex: 0, sectionPath: [] };
  }
  const rootIndex = 0;
  const root = blueprint.roots[rootIndex];
  if (!root.sections?.length) {
    return { rootIndex, sectionPath: [] };
  }
  return { rootIndex, sectionPath: [0] };
}

function aggregateErrors(
  nodes: TreeNode<SectionPath>[],
  errors: Map<string, string>,
) {
  const counts = new Map<string, number>();
  const walk = (node: TreeNode<SectionPath>): number => {
    let total = node.fieldPointers.reduce(
      (sum, pointer) => sum + (errors.has(pointer) ? 1 : 0),
      0,
    );
    node.children.forEach((child) => {
      total += walk(child);
    });
    counts.set(node.id, total);
    return total;
  };
  nodes.forEach((node) => walk(node));
  return counts;
}
