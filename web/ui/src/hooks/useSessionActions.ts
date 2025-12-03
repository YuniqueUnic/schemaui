/**
 * Session Actions Hook
 * 
 * Handles save, exit, validation, and preview logic.
 */

import { useCallback, useRef, useEffect } from "react";
import { toast } from "sonner";
import {
  exitSession,
  fetchSession,
  persistData,
  renderPreview,
  validateData,
} from "../api";
import type { JsonValue } from "../types";
import { applyUiDefaults } from "../ui-ast";
import { deepClone, setPointerValue } from "../utils/jsonPointer";
import type { useSessionState } from "./useSessionState";

type SessionStateHook = ReturnType<typeof useSessionState>;

interface UseSessionActionsOptions {
  state: SessionStateHook["state"];
  actions: SessionStateHook["actions"];
  dirtyRef: SessionStateHook["dirtyRef"];
}

export function useSessionActions({ state, actions, dirtyRef }: UseSessionActionsOptions) {
  const validationSeq = useRef(0);
  const previewSeq = useRef(0);
  const sessionIdRef = useRef("");

  // ============================================
  // localStorage Helpers
  // ============================================
  
  const getStorageKey = useCallback(() => {
    return `schemaui-session-${sessionIdRef.current}`;
  }, []);

  const saveToLocalStorage = useCallback((data: JsonValue) => {
    try {
      const key = getStorageKey();
      localStorage.setItem(key, JSON.stringify(data));
    } catch (err) {
      console.error("Failed to save to localStorage", err);
    }
  }, [getStorageKey]);

  const clearLocalStorage = useCallback(() => {
    try {
      const key = getStorageKey();
      localStorage.removeItem(key);
    } catch (err) {
      console.error("Failed to clear localStorage", err);
    }
  }, [getStorageKey]);

  // ============================================
  // Validation & Preview
  // ============================================

  const runValidation = useCallback(async (data: JsonValue): Promise<Map<string, string>> => {
    const seq = ++validationSeq.current;
    try {
      const result = await validateData(data);
      if (seq !== validationSeq.current) return new Map();
      const errors = new Map<string, string>();
      result.errors?.forEach((err) => errors.set(err.pointer || "", err.message));
      actions.setErrors(errors);
      return errors;
    } catch (error) {
      console.error("Validation failed", error);
      return new Map();
    }
  }, [actions]);

  const updatePreview = useCallback(async (data: JsonValue, pretty: boolean, format: string) => {
    const seq = ++previewSeq.current;
    try {
      const result = await renderPreview(data, format, pretty);
      if (seq !== previewSeq.current) return;
      actions.setPreviewPayload(result.payload);
    } catch (error) {
      console.error("Preview failed", error);
    }
  }, [actions]);

  // ============================================
  // Initialize Session
  // ============================================

  const initializeSession = useCallback(async () => {
    try {
      const payload = await fetchSession();
      
      // Generate session ID from title
      const titleKey = payload.title?.replace(/\s+/g, "_").replace(/[^\w-]/g, "") || "default";
      sessionIdRef.current = titleKey;

      // Try to restore from localStorage
      let restoredData: JsonValue | null = null;
      try {
        const stored = localStorage.getItem(getStorageKey());
        if (stored) {
          restoredData = JSON.parse(stored);
          toast.info("Restored previous session data");
        }
      } catch (err) {
        console.error("Failed to restore from localStorage", err);
      }

      const withDefaults = applyUiDefaults(
        payload.ui_ast,
        restoredData || payload.data || {}
      );

      const formats = payload.formats?.length ? payload.formats : ["json"];
      const initialPointer = resolveInitialPointer(payload.ui_ast);

      actions.initSession(payload, withDefaults, formats, initialPointer);

      // Run initial validation and preview
      await Promise.all([
        runValidation(withDefaults),
        updatePreview(withDefaults, true, formats.includes("json") ? "json" : formats[0]),
      ]);
    } catch (error) {
      console.error("Failed to load session", error);
      actions.setStatus("Failed to load session");
      actions.setLoading(false);
    }
  }, [actions, getStorageKey, runValidation, updatePreview]);

  // ============================================
  // Handle Data Change
  // ============================================

  const handleChange = useCallback((pointer: string, value: JsonValue) => {
    const newData = setPointerValue(state.data, pointer, deepClone(value));
    actions.updateDataAndDirty(newData);
    saveToLocalStorage(newData);
    runValidation(newData);
    updatePreview(newData, state.previewPretty, state.previewFormat);
  }, [state.data, state.previewPretty, state.previewFormat, actions, saveToLocalStorage, runValidation, updatePreview]);

  // ============================================
  // Handle Save
  // ============================================

  const handleSave = useCallback(async () => {
    if (!state.session) return;
    
    if (state.errors.size > 0) {
      toast.error(
        `Cannot save: ${state.errors.size} validation error${state.errors.size > 1 ? "s" : ""} found.`,
        {
          duration: 5000,
          action: {
            label: "View Errors",
            onClick: () => actions.setShowErrorsDialog(true),
          },
        }
      );
      actions.setShowErrorsDialog(true);
      return;
    }

    actions.setSaving(true);
    try {
      await persistData(state.data);
      actions.markSaved();
      clearLocalStorage();
      toast.success("Changes saved successfully");
    } catch (error) {
      console.error("Save failed", error);
      toast.error("Failed to save changes");
      actions.setSaving(false);
    }
  }, [state.session, state.data, state.errors, actions, clearLocalStorage]);

  // ============================================
  // Handle Exit
  // ============================================

  const handleExit = useCallback(async (force = false) => {
    // Prevent multiple exit attempts
    if (state.sessionEnded || state.exiting) return;

    // Force exit: skip all checks, discard changes
    if (force) {
      actions.setExiting(true);
      try {
        await exitSession(state.data, false); // commit=false
        clearLocalStorage();
        actions.setSessionEnded(true);
        toast.success("Session aborted (changes discarded)");
      } catch (error) {
        console.error("Exit failed", error);
        toast.error("Failed to exit session");
        actions.setExiting(false);
      }
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
          onClick: () => handleExit(true),
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
          onClick: () => handleExit(true),
        },
      });
      return;
    }

    // Clean exit: no errors, no unsaved changes
    actions.setExiting(true);
    try {
      await exitSession(state.data, true); // commit=true
      clearLocalStorage();
      actions.setSessionEnded(true);
      toast.success("Session ended successfully");
    } catch (error) {
      console.error("Exit failed", error);
      toast.error("Failed to exit session");
      actions.setExiting(false);
    }
  }, [state.sessionEnded, state.exiting, state.data, dirtyRef, actions, clearLocalStorage, runValidation]);

  // ============================================
  // Handle Preview Format/Pretty Change
  // ============================================

  const handlePreviewFormatChange = useCallback((format: string) => {
    actions.setPreviewFormat(format);
    updatePreview(state.data, state.previewPretty, format);
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

function resolveInitialPointer(ast: { roots: { pointer: string }[] } | null | undefined): string {
  return ast?.roots?.[0]?.pointer ?? "";
}
