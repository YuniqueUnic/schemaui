import { afterEach, describe, expect, it, vi } from "vitest";
import { ConnectionLostError, errorMessage, send } from "./requestError";

describe("send", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("passes a live server's response through untouched, errors included", async () => {
    const response = new Response("nope", { status: 500 });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response));

    await expect(send("/api/v1/save")).resolves.toBe(response);
  });

  it("names a rejected fetch for what it is: the server is gone", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockRejectedValue(new TypeError("Failed to fetch")),
    );

    await expect(send("/api/v1/preview")).rejects.toBeInstanceOf(
      ConnectionLostError,
    );
  });

  it("does not hide failures that are not transport failures", async () => {
    const aborted = new DOMException("aborted", "AbortError");
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(aborted));

    await expect(send("/api/v1/preview")).rejects.toBe(aborted);
  });
});

describe("errorMessage", () => {
  it("prefers the server's error field over the raw JSON body", () => {
    const error = new Error(JSON.stringify({ error: "failed to write draft" }));

    expect(errorMessage(error, "fallback")).toBe("failed to write draft");
  });

  it("shows plain-text bodies as they came", () => {
    expect(errorMessage(new Error("Bad Gateway"), "fallback")).toBe(
      "Bad Gateway",
    );
  });

  it("keeps JSON bodies that carry no usable error field readable", () => {
    const body = JSON.stringify({ status: "nope" });

    expect(errorMessage(new Error(body), "fallback")).toBe(body);
    expect(errorMessage(new Error(JSON.stringify({ error: "  " })), "x")).toBe(
      JSON.stringify({ error: "  " }),
    );
  });

  it("falls back when there is nothing to show", () => {
    expect(errorMessage(new Error(""), "fallback")).toBe("fallback");
    expect(errorMessage(new Error("   "), "fallback")).toBe("fallback");
    expect(errorMessage("a string, not an Error", "fallback")).toBe("fallback");
    expect(errorMessage(undefined, "fallback")).toBe("fallback");
  });

  it("tells the user a lost connection is not retryable", () => {
    const message = errorMessage(new ConnectionLostError(), "fallback");

    expect(message).toMatch(/lost connection/i);
    expect(message).toMatch(/restart/i);
  });
});
