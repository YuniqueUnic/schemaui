import { describe, expect, it } from "vitest";
import { richContentId } from "./richId";

// The hex values come from the Rust fixtures in `src/tests/rich/id_tests.rs`.
// They are a cross-language contract: if this file and the Rust file ever
// disagree, every figure lookup silently misses and everything falls back to
// source. Change the hash, change both.
describe("richContentId", () => {
  it("matches the Rust fixture vectors", () => {
    expect(richContentId("mermaid", "flowchart LR; A-->B")).toBe("4e445939aa84dc36");
    expect(richContentId("svg", "<svg/>")).toBe("998e073cd59a6064");
    expect(richContentId("markdown", "# hi")).toBe("ae1b2b51e60236b4");
  });

  it("distinguishes kinds over the same bytes", () => {
    expect(richContentId("svg", "flowchart LR; A-->B")).toBe("0552179fc8ada6fb");
    expect(richContentId("svg", "flowchart LR; A-->B")).not.toBe(
      richContentId("mermaid", "flowchart LR; A-->B"),
    );
  });
});
