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
  description: "Session description",
  data: {},
  formats: ["json"],
  expires_in_ms: null,
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

/** Two forms whose titles differ only in the non-ASCII part. */
function sessionWithTitleAndField(pointer: string): SessionResponse {
  return {
    ...session,
    title: "种田游戏 · 第 1 轮：定位与边界",
    ui_ast: {
      roots: [
        {
          ...session.ui_ast.roots[0],
          pointer,
          title: pointer,
        },
      ],
    },
  };
}

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
    localStorage.clear();
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

  it("shows a preview error instead of keeping the previous payload", async () => {
    apiMocks.fetchSession.mockResolvedValue({
      ...session,
      formats: ["json", "toml"],
    });
    apiMocks.validateData.mockResolvedValue({ errors: [] });
    apiMocks.renderPreview
      .mockResolvedValueOnce({ payload: '{\n  "field": ""\n}' })
      .mockRejectedValueOnce(
        new Error(JSON.stringify({ error: "TOML cannot represent null at /items/1" })),
      );

    const { result } = renderHook(() => useHarness());

    await act(async () => {
      await result.current.initializeSession();
    });

    expect(result.current.state.previewPayload).toContain("field");
    expect(result.current.state.previewError).toBeNull();

    await act(async () => {
      await result.current.handlePreviewFormatChange("toml");
    });

    expect(apiMocks.renderPreview).toHaveBeenLastCalledWith(
      { field: "" },
      "toml",
      true,
    );
    expect(result.current.state.previewPayload).toBe("");
    expect(result.current.state.previewError).toBe(
      "TOML cannot represent null at /items/1",
    );
  });

  it("keys drafts by form identity, so titles that slug alike cannot collide", async () => {
    apiMocks.validateData.mockResolvedValue({ errors: [] });

    apiMocks.fetchSession.mockResolvedValue(sessionWithTitleAndField("/field"));
    const first = renderHook(() => useHarness());

    await act(async () => {
      await first.result.current.initializeSession();
    });
    act(() => {
      first.result.current.handleChange("/field", "draft-from-first-form");
    });

    expect(Object.keys(localStorage)).toHaveLength(1);

    // A different form whose title slugifies to the same string must start
    // empty rather than inherit the first form's draft.
    apiMocks.fetchSession.mockResolvedValue(
      sessionWithTitleAndField("/other_field"),
    );
    const second = renderHook(() => useHarness());

    await act(async () => {
      await second.result.current.initializeSession();
    });

    expect(toast.info).not.toHaveBeenCalled();
    expect(second.result.current.state.data).not.toHaveProperty("field");

    act(() => {
      second.result.current.handleChange("/other_field", "draft-from-second-form");
    });

    expect(Object.keys(localStorage)).toHaveLength(2);
  });
});
