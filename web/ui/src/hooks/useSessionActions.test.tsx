import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { SessionResponse } from "../types";
import { useSessionActions } from "./useSessionActions";
import { useSessionState } from "./useSessionState";

const apiMocks = vi.hoisted(() => ({
  persistData: vi.fn(),
  validateData: vi.fn(),
  renderPreview: vi.fn(),
  fetchSession: vi.fn(),
  exitSession: vi.fn(),
}));

vi.mock("../api", () => ({
  persistData: apiMocks.persistData,
  validateData: apiMocks.validateData,
  renderPreview: apiMocks.renderPreview,
  fetchSession: apiMocks.fetchSession,
  exitSession: apiMocks.exitSession,
}));

const toast = vi.hoisted(() => ({
  error: vi.fn(),
  info: vi.fn(),
  success: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast,
}));

function useHarness() {
  const sessionState = useSessionState();
  const sessionActions = useSessionActions(sessionState);
  return {
    ...sessionState,
    ...sessionActions,
  };
}

const session: SessionResponse = {
  title: "Web Save Validation",
  data: {},
  formats: ["json"],
  ui_ast: {
    roots: [
      {
        pointer: "/field",
        title: "field",
        description: null,
        required: false,
        default_value: "",
        kind: {
          type: "field",
          scalar: "string",
          enum_options: null,
          enum_values: null,
        },
      },
    ],
  },
  layout: null,
};

describe("useSessionActions", () => {
  beforeEach(() => {
    apiMocks.persistData.mockReset();
    apiMocks.validateData.mockReset();
    apiMocks.renderPreview.mockReset();
    apiMocks.fetchSession.mockReset();
    apiMocks.exitSession.mockReset();
    toast.error.mockReset();
    toast.info.mockReset();
    toast.success.mockReset();
    toast.warning.mockReset();
    apiMocks.renderPreview.mockResolvedValue({ payload: "{}" });
  });

  it("revalidates before save and blocks stale invalid data", async () => {
    apiMocks.validateData.mockResolvedValue({
      errors: [{ pointer: "/field", message: "must match schema" }],
    });
    apiMocks.persistData.mockResolvedValue({});

    const { result } = renderHook(() => useHarness());

    act(() => {
      result.current.actions.initSession(
        session,
        { field: "bad-value" },
        ["json"],
        "/field",
      );
    });

    await act(async () => {
      await result.current.handleSave();
    });

    expect(apiMocks.validateData).toHaveBeenCalledTimes(1);
    expect(apiMocks.persistData).not.toHaveBeenCalled();
    expect(result.current.state.errors.get("/field")).toBe("must match schema");
    expect(result.current.state.showErrorsDialog).toBe(true);
    expect(toast.error).toHaveBeenCalledOnce();
  });
});
