/**
 * Session Actions Hook
 *
 * Handles save, exit, validation, and preview logic.
 */

import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import { httpTransport } from "../transport/httpTransport";
import type { SchemaUiTransport } from "../transport/types";
import type { JsonValue, UiAst } from "../types";
import { applyUiDefaults } from "../ui-ast";
import { downloadTextFile } from "../utils/downloadFile";
import { deepClone, setPointerValue } from "../utils/jsonPointer";
import type { useSessionState } from "./useSessionState";
import { staticSession } from "../generated/session";

type SessionStateHook = ReturnType<typeof useSessionState>;

interface UseSessionActionsOptions {
  state: SessionStateHook["state"];
  actions: SessionStateHook["actions"];
  dirtyRef: SessionStateHook["dirtyRef"];
  /** Where the schema pipeline actually runs. Defaults to the HTTP session
   * this hook was written for; the Playground passes a wasm-backed one. */
  transport?: SchemaUiTransport;
}

export function useSessionActions(
  { state, actions, dirtyRef, transport = httpTransport }: UseSessionActionsOptions,
) {
  const validationSeq = useRef(0);
  const previewSeq = useRef(0);
  const draftKeyRef = useRef("");

  // ============================================
  // localStorage Helpers
  // ============================================

  const saveToLocalStorage = useCallback((data: JsonValue) => {
    if (!draftKeyRef.current) return;
    try {
      localStorage.setItem(draftKeyRef.current, JSON.stringify(data));
    } catch (err) {
      console.error("Failed to save to localStorage", err);
    }
  }, []);

  const clearLocalStorage = useCallback(() => {
    if (!draftKeyRef.current) return;
    try {
      localStorage.removeItem(draftKeyRef.current);
    } catch (err) {
      console.error("Failed to clear localStorage", err);
    }
  }, []);

  // ============================================
  // Validation & Preview
  // ============================================

  const runValidation = useCallback(
    async (data: JsonValue): Promise<Map<string, string>> => {
      const seq = ++validationSeq.current;
      try {
        const result = await transport.validate(data);
        if (seq !== validationSeq.current) return new Map();
        const errors = new Map<string, string>();
        result.errors?.forEach((err) =>
          errors.set(err.pointer || "", err.message)
        );
        actions.setErrors(errors);
        return errors;
      } catch (error) {
        console.error("Validation failed", error);
        return new Map();
      }
    },
    [actions, transport],
  );

  const updatePreview = useCallback(
    async (data: JsonValue, pretty: boolean, format: string) => {
      const seq = ++previewSeq.current;
      try {
        const result = await transport.preview(data, format, pretty);
        if (seq !== previewSeq.current) return;
        actions.setPreviewPayload(result.payload);
      } catch (error) {
        if (seq !== previewSeq.current) return;
        const message = previewErrorMessage(error);
        console.error("Preview failed", error);
        actions.setPreviewError(message);
      }
    },
    [actions, transport],
  );

  // ============================================
  // Initialize Session
  // ============================================

  const initializeSession = useCallback(async () => {
    try {
      const payload = staticSession ?? await transport.bootstrap();

      draftKeyRef.current = draftStorageKey(payload.ui_ast);

      // Inject user theme stylesheet when the backend offers one.
      // A <link> is idempotent-safe: if the element already exists we skip it,
      // so hot-reloading the hook does not pile up duplicate stylesheets.
      if (payload.capabilities?.includes("theme")) {
        const THEME_LINK_ID = "schemaui-user-theme";
        if (!document.getElementById(THEME_LINK_ID)) {
          const link = document.createElement("link");
          link.id = THEME_LINK_ID;
          link.rel = "stylesheet";
          link.href = "/api/v1/theme.css";
          document.head.appendChild(link);
        }
      }

      // Two independent draft layers:
      //   1. Backend draft (draft_restored=true): the server already merged it
      //      into payload.data. We just show the user a banner.
      //   2. Browser localStorage: a crash-recovery layer written on every
      //      keystroke, cleared only on a successful commit exit. Preferred
      //      over the backend draft because it is more recent.
      let effectiveData: JsonValue = payload.data || {};

      if (payload.draft_restored) {
        toast.info("Draft restored from previous session", { duration: 6000 });
      }

      try {
        const stored = localStorage.getItem(draftKeyRef.current);
        if (stored) {
          effectiveData = JSON.parse(stored) as JsonValue;
          toast.info("Unsaved changes restored from browser storage", { duration: 4000 });
        }
      } catch (err) {
        console.error("Failed to restore from localStorage", err);
      }

      const withDefaults = applyUiDefaults(payload.ui_ast, effectiveData);

      const formats = payload.formats?.length ? payload.formats : ["json"];

      // Land on the virtual root, so the first thing on screen is the whole
      // document rather than whichever section happened to be declared first.
      // Every section is reachable from there in one scroll, and the tabs are
      // there for anyone who would rather narrow it down.
      actions.initSession(payload, withDefaults, formats, ROOT_POINTER);

      // Run initial validation and preview
      await Promise.all([
        runValidation(withDefaults),
        updatePreview(
          withDefaults,
          true,
          formats.includes("json") ? "json" : formats[0],
        ),
      ]);
    } catch (error) {
      console.error("Failed to load session", error);
      actions.setStatus("Failed to load session");
      actions.setLoading(false);
    }
  }, [actions, runValidation, updatePreview, transport]);

  // ============================================
  // Handle Data Change
  // ============================================

  const handleChange = useCallback((pointer: string, value: JsonValue) => {
    const newData = setPointerValue(state.data, pointer, deepClone(value));
    actions.updateDataAndDirty(newData);
    saveToLocalStorage(newData);
    runValidation(newData);
    updatePreview(newData, state.previewPretty, state.previewFormat);
  }, [
    state.data,
    state.previewPretty,
    state.previewFormat,
    actions,
    saveToLocalStorage,
    runValidation,
    updatePreview,
  ]);

  // ============================================
  // Handle Save
  // ============================================

  const handleSave = useCallback(async () => {
    if (!state.session) return;

    const validationErrors = await runValidation(state.data);
    if (validationErrors.size > 0) {
      toast.error(
        `Cannot save: ${validationErrors.size} validation error${
          validationErrors.size > 1 ? "s" : ""
        } found.`,
        {
          duration: 5000,
          action: {
            label: "View Errors",
            onClick: () => actions.setShowErrorsDialog(true),
          },
        },
      );
      actions.setShowErrorsDialog(true);
      return;
    }

    actions.setSaving(true);
    try {
      await transport.save(state.data);
      actions.markSaved();
      // localStorage is NOT cleared here: it is an independent crash-recovery
      // layer that protects against power-cuts and process crashes. A backend
      // save writes to disk on the server side; the browser draft guards against
      // the opposite failure. Both layers can co-exist. localStorage is only
      // cleared once the session ends with a successful commit.
      toast.success(
        transport.kind === "wasm"
          ? "Kept in this browser only — there is no server to save to"
          : "Draft saved",
      );
    } catch (error) {
      console.error("Save failed", error);
      toast.error("Failed to save draft");
      actions.setSaving(false);
    }
  }, [state.session, state.data, actions, runValidation, transport]);

  // ============================================
  // Handle Exit
  // ============================================

  const forceExit = useCallback(async () => {
    actions.setExiting(true);
    try {
      await transport.exit(state.data, false); // commit=false
      clearLocalStorage();
      actions.setSessionEnded(true);
      toast.success("Session aborted (changes discarded)");
    } catch (error) {
      console.error("Exit failed", error);
      toast.error("Failed to exit session");
      actions.setExiting(false);
    }
  }, [state.data, actions, clearLocalStorage, transport]);

  const handleExit = useCallback(async (force = false) => {
    // Prevent multiple exit attempts
    if (state.sessionEnded || state.exiting) return;

    // Force exit: skip all checks, discard changes
    if (force) {
      await forceExit();
      return;
    }

    // Normal exit: check for validation errors first
    const validationErrors = await runValidation(state.data);
    const hasErrors = validationErrors.size > 0;

    if (hasErrors) {
      toast.warning("Cannot exit: please fix validation errors first.", {
        duration: 10000,
        action: {
          label: "Force Exit",
          onClick: () => void forceExit(),
        },
      });
      return;
    }

    // Check for unsaved changes
    const isDirty = dirtyRef.current;
    if (isDirty) {
      toast.warning("You have unsaved changes. Please save before exiting.", {
        duration: 10000,
        action: {
          label: "Force Exit",
          onClick: () => void forceExit(),
        },
      });
      return;
    }

    // Clean exit: no errors, no unsaved changes
    actions.setExiting(true);
    try {
      await transport.exit(state.data, true); // commit=true
      // There is no server session for the wasm transport to hand the
      // finished document to, so "ending" the session means downloading it.
      if (transport.kind === "wasm") {
        const format = state.previewFormat || "json";
        downloadTextFile(`document.${format}`, state.previewPayload, previewMimeType(format));
      }
      clearLocalStorage();
      actions.setSessionEnded(true);
      toast.success(
        transport.kind === "wasm" ? "Document downloaded" : "Session ended successfully",
      );
    } catch (error) {
      console.error("Exit failed", error);
      toast.error("Failed to exit session");
      actions.setExiting(false);
    }
  }, [
    state.sessionEnded,
    state.exiting,
    state.data,
    state.previewFormat,
    state.previewPayload,
    dirtyRef,
    actions,
    clearLocalStorage,
    forceExit,
    runValidation,
    transport,
  ]);

  // ============================================
  // Handle Preview Format/Pretty Change
  // ============================================

  const handlePreviewFormatChange = useCallback((format: string) => {
    actions.setPreviewFormat(format);
    return updatePreview(state.data, state.previewPretty, format);
  }, [state.data, state.previewPretty, actions, updatePreview]);

  const handlePreviewPrettyChange = useCallback((pretty: boolean) => {
    actions.setPreviewPretty(pretty);
    updatePreview(state.data, pretty, state.previewFormat);
  }, [state.data, state.previewFormat, actions, updatePreview]);

  // ============================================
  // Keyboard Shortcuts
  // ============================================

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        handleSave();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [handleSave]);

  return {
    initializeSession,
    handleChange,
    handleSave,
    handleExit,
    handlePreviewFormatChange,
    handlePreviewPrettyChange,
  };
}

// ============================================
// Helpers
// ============================================

/**
 * localStorage key for this session's unsaved draft.
 *
 * Keyed by the UI AST rather than the title. Titles are human-facing and
 * routinely non-ASCII, and any slug that keeps only `\w` characters strips
 * them entirely, so every distinct form whose title shares a digit ("… 第 1
 * 轮：…") collapses onto one key and restores another form's draft. The AST is
 * what actually identifies the form.
 */
function draftStorageKey(ast: UiAst | null | undefined): string {
  return `schemaui-draft-${fnv1a64(JSON.stringify(ast ?? null))}`;
}

const FNV_OFFSET = 0x811c9dc5;
const FNV_PRIME = 0x01000193;

/** Two independently seeded FNV-1a lanes, joined into a 64-bit hex digest. */
function fnv1a64(text: string): string {
  let low = FNV_OFFSET;
  let high = FNV_OFFSET ^ text.length;
  for (let i = 0; i < text.length; i += 1) {
    const code = text.charCodeAt(i);
    low = Math.imul(low ^ code, FNV_PRIME);
    high = Math.imul(high ^ code, FNV_PRIME) ^ i;
  }
  return (low >>> 0).toString(16).padStart(8, "0")
    + (high >>> 0).toString(16).padStart(8, "0");
}

/**
 * The virtual root of the document.
 *
 * An empty JSON Pointer addresses the document itself, which is exactly what
 * the root view shows: every section, in order. It is a real selection, not a
 * "nothing selected" sentinel — that is why it is spelled out here rather than
 * left as a bare `""` at the call site.
 */
const ROOT_POINTER = "";

function previewErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    try {
      const parsed = JSON.parse(error.message) as { error?: unknown };
      if (typeof parsed.error === "string" && parsed.error.trim()) {
        return parsed.error;
      }
    } catch {
      return error.message;
    }
    return error.message;
  }
  return "Preview failed";
}

const PREVIEW_MIME_TYPES: Record<string, string> = {
  json: "application/json",
  yaml: "application/yaml",
  toml: "application/toml",
};

function previewMimeType(format: string): string {
  return PREVIEW_MIME_TYPES[format] ?? "text/plain";
}
