import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";
import { FieldRenderer } from "./FieldRenderer";
import { RichAssetsProvider } from "../rich/RichAssetsProvider";
import { HoverPreviewProvider } from "../rich/HoverPreviewLayer";
import { OverlayProvider } from "../Overlay";
import { ThemeProvider } from "../../theme";
import { richContentId } from "../../lib/richId";
import type { JsonValue, RichContent, UiNode, UiNodeKind } from "../../types";

type FieldNode = UiNode & { kind: Extract<UiNodeKind, { type: "field" }> };

const SVG_SOURCE = `<svg viewBox="0 0 24 24"><rect width="24" height="24"/></svg>`;
const monolithFigure: RichContent = { type: "svg", source: SVG_SOURCE };
const ASSETS = {
  [richContentId("svg", SVG_SOURCE)]: {
    kind: "svg" as const,
    svg: { body: "<svg viewBox='0 0 24 24'></svg>", width: 24, height: 24 },
  },
};

function enumField(): FieldNode {
  return {
    pointer: "/architecture",
    title: "Reference architecture",
    description: null,
    required: false,
    default_value: null,
    kind: {
      type: "field",
      scalar: "string",
      enum_options: ["Monolith", "Service mesh"],
      enum_values: ["monolith", "mesh"],
      enum_details: [
        { label: "Monolith", content: monolithFigure },
        { label: "Service mesh" },
      ],
    },
  };
}

function SelectField({ value }: { value: JsonValue | undefined }) {
  return (
    <ThemeProvider>
      <OverlayProvider>
        <HoverPreviewProvider>
          <RichAssetsProvider assets={ASSETS}>
            <FieldRenderer
              node={enumField()}
              value={value}
              onChange={() => {}}
            />
          </RichAssetsProvider>
        </HoverPreviewProvider>
      </OverlayProvider>
    </ThemeProvider>
  );
}

describe("EnumControl select with figures", () => {
  it("keeps the trigger text-only — a figure taller than the row used to overflow it", () => {
    const { container } = render(<SelectField value="monolith" />);
    const trigger = container.querySelector("button[role='combobox']")!;
    expect(trigger.textContent).toContain("Monolith");
    // No figure rides inside the trigger; lucide's chevron is the only svg.
    expect(trigger.querySelectorAll("[role='img']")).toHaveLength(0);
  });

  it("shows the selected option's figure beside the control, fully interactive", () => {
    const { container } = render(<SelectField value="monolith" />);
    const figures = container.querySelectorAll("[role='img']");
    expect(figures).toHaveLength(1);
    expect(figures[0].getAttribute("aria-label")).toBe("Monolith");
    // Full interactions: hover enlarges and click opens the viewer.
    expect(container.querySelector("span.cursor-zoom-in")).toBeTruthy();
  });

  it("shows no figure when the selected option has none", () => {
    const { container } = render(<SelectField value="mesh" />);
    expect(container.querySelectorAll("[role='img']")).toHaveLength(0);
    expect(container.querySelector("span.cursor-zoom-in")).toBeNull();
  });
});
