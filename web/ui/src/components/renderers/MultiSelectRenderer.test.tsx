import { useState } from "react";
import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { JsonValue, UiNode, UiNodeKind } from "../../types";
import type { EnumFieldKind } from "../../utils/typeHelpers";
import { MultiSelectRenderer } from "./MultiSelectRenderer";

const itemKind: EnumFieldKind = {
  type: "field",
  scalar: "string",
  enum_options: ["坐骑", "空翻", "地形"],
  enum_values: ["坐骑", "空翻", "地形"],
  nullable: false,
  multiline: false,
};

const node: UiNode & { kind: Extract<UiNodeKind, { type: "array" }> } = {
  pointer: "/signature",
  required: false,
  kind: { type: "array", item: itemKind },
};

function Harness({ initial }: { initial: JsonValue }) {
  const [value, setValue] = useState<JsonValue>(initial);
  return (
    <>
      <MultiSelectRenderer
        node={node}
        itemKind={itemKind}
        value={value}
        onChange={(_, next) => setValue(next)}
      />
      <output data-testid="value">{JSON.stringify(value)}</output>
    </>
  );
}

function currentValue(): string {
  return screen.getByTestId("value").textContent ?? "";
}

describe("MultiSelectRenderer", () => {
  it("checks the options the value already contains", () => {
    render(<Harness initial={["空翻"]} />);

    expect(screen.getByRole("checkbox", { name: "空翻" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "坐骑" })).not.toBeChecked();
  });

  it("removes an option instead of appending a duplicate when toggled twice", async () => {
    const user = userEvent.setup();
    render(<Harness initial={["地形"]} />);

    await user.click(screen.getByRole("checkbox", { name: "地形" }));
    expect(currentValue()).toBe("[]");

    await user.click(screen.getByRole("checkbox", { name: "地形" }));
    expect(currentValue()).toBe('["地形"]');
  });

  it("emits selections in schema order rather than click order", async () => {
    const user = userEvent.setup();
    render(<Harness initial={[]} />);

    await user.click(screen.getByRole("checkbox", { name: "地形" }));
    await user.click(screen.getByRole("checkbox", { name: "坐骑" }));

    expect(currentValue()).toBe('["坐骑","地形"]');
  });
});
