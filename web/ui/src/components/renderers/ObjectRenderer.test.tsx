import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/react";
import type { JsonValue, UiNode, VisibleWhen } from "../../types";
import { ObjectRenderer } from "./ObjectRenderer";

function field(pointer: string, visibleWhen?: VisibleWhen): UiNode {
  return {
    pointer,
    required: false,
    visible_when: visibleWhen ?? null,
    kind: { type: "field", scalar: "string" },
  };
}

/**
 * An object as it reaches this renderer from an array entry or a variant: the
 * entry owns the value, the node tree is shared by every entry.
 */
const entry: UiNode = {
  pointer: "/attachments/0",
  required: false,
  kind: {
    type: "object",
    required: [],
    children: [
      field("/attachments/0/kind"),
      field("/attachments/0/kind_custom", {
        field: "kind",
        op: "equals",
        value: "其他",
      }),
    ],
  },
};

/** Stub that prints the pointer of every child it is handed. */
const renderNode = vi.fn((node: UiNode) => <span>{node.pointer}</span>);

function renderEntry(value: JsonValue): string {
  const { container } = render(
    <ObjectRenderer
      node={entry}
      value={value}
      errors={new Map()}
      onChange={() => {}}
      renderNode={renderNode}
    />,
  );
  return container.textContent ?? "";
}

describe("ObjectRenderer visibility", () => {
  it("drops a child whose rule the entry does not satisfy", () => {
    const rendered = renderEntry({ kind: "发票" });

    expect(rendered).toContain("/attachments/0/kind");
    expect(rendered).not.toContain("kind_custom");
  });

  it("renders the child once the entry satisfies the rule", () => {
    const rendered = renderEntry({ kind: "其他" });

    expect(rendered).toContain("/attachments/0/kind_custom");
  });
});
