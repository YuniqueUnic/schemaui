import { afterEach, describe, expect, it, vi } from "vitest";
import { persistData, renderPreview } from "./api";
import { ConnectionLostError } from "./utils/requestError";

function stubFetch(impl: () => Promise<Response>) {
  const fetchMock = vi.fn(impl);
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("api request layer", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("posts the document to the save endpoint and returns the server's answer", async () => {
    const fetchMock = stubFetch(async () =>
      Response.json({ status: "saved" }),
    );

    await expect(persistData({ a: 1 })).resolves.toEqual({ status: "saved" });

    const [path, init] = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
    expect(path).toBe("/api/v1/save");
    expect(init.method).toBe("POST");
    expect(JSON.parse(String(init.body))).toEqual({ data: { a: 1 } });
  });

  it("saves an unchanged, empty document just the same", async () => {
    stubFetch(async () => Response.json({ status: "saved" }));

    await expect(persistData({})).resolves.toEqual({ status: "saved" });
  });

  it("raises ConnectionLostError when the server cannot be reached", async () => {
    stubFetch(async () => {
      throw new TypeError("Failed to fetch");
    });

    await expect(persistData({})).rejects.toBeInstanceOf(ConnectionLostError);
    await expect(renderPreview({}, "json", true)).rejects.toBeInstanceOf(
      ConnectionLostError,
    );
  });

  it("carries the server's error body for a failed save", async () => {
    stubFetch(
      async () =>
        new Response(JSON.stringify({ error: "failed to write draft: disk full" }), {
          status: 500,
        }),
    );

    await expect(persistData({})).rejects.toThrow(/disk full/);
  });

  it("reports a closed session as the 410 body, not as a lost connection", async () => {
    stubFetch(
      async () =>
        new Response(JSON.stringify({ error: "session already closed" }), {
          status: 410,
        }),
    );

    const failure = persistData({});
    await expect(failure).rejects.toThrow(/session already closed/);
    await expect(failure).rejects.not.toBeInstanceOf(ConnectionLostError);
  });

  it("falls back to the status code when the error body is empty", async () => {
    stubFetch(async () => new Response("", { status: 502 }));

    await expect(renderPreview({}, "json", true)).rejects.toThrow(
      "Request failed: 502",
    );
  });
});
