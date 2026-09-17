import { describe, expect, it } from "vitest";
import type { UiAst, UiNode, VisibleWhen } from "../types";
import { pruneHiddenNodes } from "./visibility";

function field(pointer: string, visibleWhen?: VisibleWhen): UiNode {
  return {
    pointer,
    required: false,
    visible_when: visibleWhen ?? null,
    kind: { type: "field", scalar: "string" },
  };
}

function section(pointer: string, children: UiNode[]): UiNode {
  return {
    pointer,
    required: false,
    kind: { type: "object", children, required: [] },
  };
}

const ast: UiAst = {
  roots: [
    section("/tech", [
      field("/tech/delivery_form"),
      field("/tech/delivery_form_custom", {
        field: "delivery_form",
        op: "equals",
        value: "其他",
      }),
    ]),
  ],
};

function visiblePointers(result: UiAst | undefined): string[] {
  const root = result?.roots[0];
  if (!root || root.kind.type !== "object") return [];
  return (root.kind.children ?? []).map((child) => child.pointer);
}

describe("pruneHiddenNodes", () => {
  it("hides a dependent node while the sibling holds another value", () => {
    const result = pruneHiddenNodes(ast, { tech: { delivery_form: "单个 HTML 文件" } });
    expect(visiblePointers(result)).toEqual(["/tech/delivery_form"]);
  });

  it("reveals the dependent node once the sibling matches", () => {
    const result = pruneHiddenNodes(ast, { tech: { delivery_form: "其他" } });
    expect(visiblePointers(result)).toEqual([
      "/tech/delivery_form",
      "/tech/delivery_form_custom",
    ]);
  });

  it("hides a dependent node when the containing object is absent", () => {
    const result = pruneHiddenNodes(ast, {});
    expect(visiblePointers(result)).toEqual(["/tech/delivery_form"]);
  });

  it("matches non-string values", () => {
    const toggleAst: UiAst = {
      roots: [
        section("/root", [
          field("/root/advanced"),
          field("/root/retries", {
            field: "advanced",
            op: "equals",
            value: true,
          }),
        ]),
      ],
    };

    expect(visiblePointers(pruneHiddenNodes(toggleAst, { root: { advanced: false } })))
      .toEqual(["/root/advanced"]);
    expect(visiblePointers(pruneHiddenNodes(toggleAst, { root: { advanced: true } })))
      .toEqual(["/root/advanced", "/root/retries"]);
  });

  it("reveals a dependent node when an array sibling contains the value", () => {
    const setAst: UiAst = {
      roots: [
        section("/features", [
          field("/features/signature"),
          field("/features/signature_custom", {
            field: "signature",
            op: "contains",
            value: "其他",
          }),
        ]),
      ],
    };

    expect(visiblePointers(pruneHiddenNodes(setAst, { features: { signature: [] } })))
      .toEqual(["/features/signature"]);
    expect(visiblePointers(
      pruneHiddenNodes(setAst, { features: { signature: ["坐骑", "其他"] } }),
    )).toEqual(["/features/signature", "/features/signature_custom"]);
    expect(visiblePointers(
      pruneHiddenNodes(setAst, { features: { signature: "其他" } }),
    )).toEqual(["/features/signature"]);
  });

  it("keeps nodes that carry no rule", () => {
    const result = pruneHiddenNodes(ast, {});
    expect(visiblePointers(result)).toContain("/tech/delivery_form");
  });

  it("returns undefined when there is no ast", () => {
    expect(pruneHiddenNodes(undefined, {})).toBeUndefined();
    expect(pruneHiddenNodes(null, {})).toBeUndefined();
  });
});
