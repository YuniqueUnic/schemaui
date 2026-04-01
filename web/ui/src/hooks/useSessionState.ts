/**
 * Session State Management Hook
 *
 * Centralizes all session-related state using useReducer for predictable updates.
 */

import { useCallback, useEffect, useReducer, useRef } from "react";
import type { JsonValue, SessionResponse } from "../types";

// ============================================
// State Types
// ============================================

export interface SessionState {
  // Core data
  session: SessionResponse | null;
  data: JsonValue;

  // UI state
  selectedPointer: string;
  errors: Map<string, string>;

  // Status flags
  loading: boolean;
  dirty: boolean;
  saving: boolean;
  exiting: boolean;
  sessionEnded: boolean;

  // Preview
  formats: string[];
  previewFormat: string;
  previewPretty: boolean;
  previewPayload: string;

  // Dialogs
  showErrorsDialog: boolean;

  // Status message
  status: string;
}

// ============================================
// Actions
// ============================================

type Action =
  | {
    type: "INIT_SESSION";
    payload: {
      session: SessionResponse;
      data: JsonValue;
      formats: string[];
      pointer: string;
    };
  }
  | { type: "SET_DATA"; payload: JsonValue }
  | { type: "SET_DIRTY"; payload: boolean }
  | { type: "SET_ERRORS"; payload: Map<string, string> }
  | { type: "SET_SELECTED_POINTER"; payload: string }
  | { type: "SET_LOADING"; payload: boolean }
  | { type: "SET_SAVING"; payload: boolean }
  | { type: "SET_EXITING"; payload: boolean }
  | { type: "SET_SESSION_ENDED"; payload: boolean }
  | { type: "SET_PREVIEW_FORMAT"; payload: string }
  | { type: "SET_PREVIEW_PRETTY"; payload: boolean }
  | { type: "SET_PREVIEW_PAYLOAD"; payload: string }
  | { type: "SET_SHOW_ERRORS_DIALOG"; payload: boolean }
  | { type: "SET_STATUS"; payload: string }
  | { type: "MARK_SAVED" }
  | { type: "UPDATE_DATA_AND_DIRTY"; payload: JsonValue };

// ============================================
// Initial State
// ============================================

const initialState: SessionState = {
  session: null,
  data: {},
  selectedPointer: "",
  errors: new Map(),
  loading: true,
  dirty: false,
  saving: false,
  exiting: false,
  sessionEnded: false,
  formats: ["json"],
  previewFormat: "json",
  previewPretty: true,
  previewPayload: "{}",
  showErrorsDialog: false,
  status: "Loading schema…",
};

// ============================================
// Reducer
// ============================================

function sessionReducer(state: SessionState, action: Action): SessionState {
  switch (action.type) {
    case "INIT_SESSION":
      return {
        ...state,
        session: action.payload.session,
        data: action.payload.data,
        formats: action.payload.formats,
        selectedPointer: action.payload.pointer,
        previewFormat: action.payload.formats.includes("json")
          ? "json"
          : action.payload.formats[0],
        loading: false,
        status: "Ready",
      };

    case "SET_DATA":
      return { ...state, data: action.payload };

    case "SET_DIRTY":
      return { ...state, dirty: action.payload };

    case "UPDATE_DATA_AND_DIRTY":
      return { ...state, data: action.payload, dirty: true };

    case "SET_ERRORS":
      return { ...state, errors: action.payload };

    case "SET_SELECTED_POINTER":
      return { ...state, selectedPointer: action.payload };

    case "SET_LOADING":
      return { ...state, loading: action.payload };

    case "SET_SAVING":
      return { ...state, saving: action.payload };

    case "SET_EXITING":
      return { ...state, exiting: action.payload };

    case "SET_SESSION_ENDED":
      return { ...state, sessionEnded: action.payload };

    case "SET_PREVIEW_FORMAT":
      return { ...state, previewFormat: action.payload };

    case "SET_PREVIEW_PRETTY":
      return { ...state, previewPretty: action.payload };

    case "SET_PREVIEW_PAYLOAD":
      return { ...state, previewPayload: action.payload };

    case "SET_SHOW_ERRORS_DIALOG":
      return { ...state, showErrorsDialog: action.payload };

    case "SET_STATUS":
      return { ...state, status: action.payload };

    case "MARK_SAVED":
      return { ...state, dirty: false, saving: false };

    default:
      return state;
  }
}

// ============================================
// Hook
// ============================================

export function useSessionState() {
  const [state, dispatch] = useReducer(sessionReducer, initialState);

  // Use ref to always have latest dirty value (avoids stale closure issues)
  const dirtyRef = useRef(false);

  useEffect(() => {
    dirtyRef.current = state.dirty;
  }, [state.dirty]);

  // Action creators
  const actions = {
    initSession: useCallback(
      (
        session: SessionResponse,
        data: JsonValue,
        formats: string[],
        pointer: string,
      ) => {
        dispatch({
          type: "INIT_SESSION",
          payload: { session, data, formats, pointer },
        });
      },
      [],
    ),

    setData: useCallback((data: JsonValue) => {
      dispatch({ type: "SET_DATA", payload: data });
    }, []),

    updateDataAndDirty: useCallback((data: JsonValue) => {
      dirtyRef.current = true;
      dispatch({ type: "UPDATE_DATA_AND_DIRTY", payload: data });
    }, []),

    setDirty: useCallback((dirty: boolean) => {
      dirtyRef.current = dirty;
      dispatch({ type: "SET_DIRTY", payload: dirty });
    }, []),

    setErrors: useCallback((errors: Map<string, string>) => {
      dispatch({ type: "SET_ERRORS", payload: errors });
    }, []),

    setSelectedPointer: useCallback((pointer: string) => {
      dispatch({ type: "SET_SELECTED_POINTER", payload: pointer });
    }, []),

    setLoading: useCallback((loading: boolean) => {
      dispatch({ type: "SET_LOADING", payload: loading });
    }, []),

    setSaving: useCallback((saving: boolean) => {
      dispatch({ type: "SET_SAVING", payload: saving });
    }, []),

    setExiting: useCallback((exiting: boolean) => {
      dispatch({ type: "SET_EXITING", payload: exiting });
    }, []),

    setSessionEnded: useCallback((ended: boolean) => {
      dispatch({ type: "SET_SESSION_ENDED", payload: ended });
    }, []),

    setPreviewFormat: useCallback((format: string) => {
      dispatch({ type: "SET_PREVIEW_FORMAT", payload: format });
    }, []),

    setPreviewPretty: useCallback((pretty: boolean) => {
      dispatch({ type: "SET_PREVIEW_PRETTY", payload: pretty });
    }, []),

    setPreviewPayload: useCallback((payload: string) => {
      dispatch({ type: "SET_PREVIEW_PAYLOAD", payload: payload });
    }, []),

    setShowErrorsDialog: useCallback((show: boolean) => {
      dispatch({ type: "SET_SHOW_ERRORS_DIALOG", payload: show });
    }, []),

    setStatus: useCallback((status: string) => {
      dispatch({ type: "SET_STATUS", payload: status });
    }, []),

    markSaved: useCallback(() => {
      dirtyRef.current = false;
      dispatch({ type: "MARK_SAVED" });
    }, []),
  };

  return {
    state,
    actions,
    dirtyRef,
  };
}
