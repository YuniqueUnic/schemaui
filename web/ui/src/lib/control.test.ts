import { describe, expect, it } from "vitest";
import type { ScalarKind, UiNode, UiNodeKind } from "../types";
import {
  defaultControl,
  resolveBounds,
  resolveControl,
  snapToBounds,
  trackPercent,
} from "./control";

/** A field node with only the parts the resolver reads. */
function field(
  scalar: ScalarKind,
  extra: Partial<{
    control: UiNode["control"];
    bounds: UiNode["bounds"];
    enumOptions: string[];
    multiline: boolean;
  }> = {},
): UiNode {
  const kind: UiNodeKind = {
    type: "field",
    scalar,
    enum_options: extra.enumOptions ?? null,
    multiline: extra.multiline ?? false,
  };
  return {
    pointer: "/field",
    required: false,
    control: extra.control ?? null,
    bounds: extra.bounds ?? null,
    kind,
  };
}

/** A two-number array node, the shape `range` requires. */
function pair(
  extra: Partial<{ control: UiNode["control"]; bounds: UiNode["bounds"] }> = {},
): UiNode {
  return {
    pointer: "/window",
    required: false,
    control: extra.control ?? null,
    bounds: extra.bounds ?? null,
    kind: {
      type: "array",
      item: { type: "field", scalar: "number" },
      min_items: 2,
      max_items: 2,
    },
  };
}

const bounded = { minimum: 0, maximum: 100, step: null, marks: [] };

describe("defaultControl", () => {
  it("picks the control that suits each scalar", () => {
    expect(defaultControl(field("string"))).toBe("text");
    expect(defaultControl(field("string", { multiline: true }))).toBe(
      "textarea",
    );
    expect(defaultControl(field("boolean"))).toBe("switch");
    expect(defaultControl(field("integer"))).toBe("number");
    expect(defaultControl(field("number"))).toBe("number");
  });

  it("prefers a select for an enum, whatever the scalar is", () => {
    expect(defaultControl(field("string", { enumOptions: ["a", "b"] }))).toBe(
      "select",
    );
    expect(defaultControl(field("integer", { enumOptions: ["1"] }))).toBe(
      "select",
    );
  });

  it("falls back to a number for a node that is not a field", () => {
    // Arrays and objects are rendered by their own renderers; the value here is
    // never used, but it has to be something a caller can switch on.
    expect(defaultControl(pair())).toBe("number");
  });
});

describe("resolveControl", () => {
  it("honours a request that fits the value", () => {
    expect(resolveControl(field("string", { control: "color" }))).toBe("color");
    expect(resolveControl(field("boolean", { control: "checkbox" }))).toBe(
      "checkbox",
    );
    expect(
      resolveControl(
        field("string", { control: "segmented", enumOptions: ["a"] }),
      ),
    ).toBe("segmented");
    expect(
      resolveControl(field("number", { control: "slider", bounds: bounded })),
    )
      .toBe("slider");
    expect(resolveControl(pair({ control: "range", bounds: bounded }))).toBe(
      "range",
    );
  });

  it("falls back when the requested control does not fit the value", () => {
    // The schema and the frontend version independently, so a mismatch has to
    // produce a usable field rather than an empty card.
    expect(resolveControl(field("string", { control: "slider" }))).toBe("text");
    expect(resolveControl(field("boolean", { control: "color" }))).toBe(
      "switch",
    );
    expect(resolveControl(field("number", { control: "textarea" }))).toBe(
      "number",
    );
    expect(resolveControl(field("string", { control: "segmented" }))).toBe(
      "text",
    );
  });

  it("declines a slider with no bounds and shows a number box instead", () => {
    // A track with one end would mean inventing the other, and an invented
    // maximum is a worse answer than a plain number input.
    expect(resolveControl(field("number", { control: "slider" }))).toBe(
      "number",
    );
    expect(
      resolveControl(
        field("number", { control: "slider", bounds: { minimum: 0 } }),
      ),
    ).toBe("number");
    expect(
      resolveControl(
        field("number", { control: "slider", bounds: { maximum: 10 } }),
      ),
    ).toBe("number");
  });

  it("declines a range with no bounds", () => {
    expect(resolveControl(pair({ control: "range" }))).toBe("number");
  });

  it("declines a range whose bounds do not form an interval", () => {
    expect(
      resolveControl(
        pair({ control: "range", bounds: { minimum: 10, maximum: 10 } }),
      ),
    ).toBe("number");
    expect(
      resolveControl(
        pair({ control: "range", bounds: { minimum: 10, maximum: 0 } }),
      ),
    ).toBe("number");
  });

  it("does not offer a range for an array that is not a pair", () => {
    const triple: UiNode = {
      pointer: "/window",
      required: false,
      control: "range",
      bounds: bounded,
      kind: {
        type: "array",
        item: { type: "field", scalar: "number" },
        min_items: 3,
        max_items: 3,
      },
    };
    expect(resolveControl(triple)).toBe("number");
  });

  it("leaves a node with no request on its default", () => {
    expect(resolveControl(field("string"))).toBe("text");
    expect(resolveControl(field("boolean"))).toBe("switch");
  });
});

describe("resolveBounds", () => {
  it("returns null when either end is missing", () => {
    expect(resolveBounds(field("number"))).toBeNull();
    expect(resolveBounds(field("number", { bounds: { minimum: 0 } })))
      .toBeNull();
    expect(resolveBounds(field("number", { bounds: { maximum: 0 } })))
      .toBeNull();
  });

  it("defaults the step to one whole unit on an integer range", () => {
    const bounds = resolveBounds(
      field("integer", { bounds: { minimum: 0, maximum: 10 } }),
    );
    expect(bounds?.step).toBe(1);
  });

  it("asks the item kind for an integer range, not the node kind", () => {
    // A `range` node is an *array* of two numbers; its own kind is never a
    // field. Testing only the node kind here made every integer range fall
    // through to the fractional guess and offer values like 4.6.
    const integerPair: UiNode = {
      ...pair({ bounds: { minimum: 0, maximum: 24, step: null, marks: [] } }),
      kind: {
        type: "array",
        item: { type: "field", scalar: "integer" },
        min_items: 2,
        max_items: 2,
      },
    };
    expect(resolveBounds(integerPair)?.step).toBe(1);

    // The fractional item keeps the fractional guess.
    expect(resolveBounds(pair({ bounds: { minimum: 0, maximum: 1 } }))?.step)
      .toBe(0.01);
  });

  it("derives a fractional step for a fractional range", () => {
    // A step of 1 across 0–1 would leave all but two positions unreachable.
    const bounds = resolveBounds(
      field("number", { bounds: { minimum: 0, maximum: 1 } }),
    );
    expect(bounds?.step).toBe(0.01);
  });

  it("derives a whole step once the span is wide enough", () => {
    // The guess depends on the span, not on whether the ends look whole: 0–100
    // as a `number` still steps by 1.
    expect(
      resolveBounds(field("number", { bounds: { minimum: 0, maximum: 100 } }))
        ?.step,
    ).toBe(1);
    expect(
      resolveBounds(field("number", { bounds: { minimum: 0, maximum: 1000 } }))
        ?.step,
    ).toBe(10);
  });

  it("steps an integer by whole units whatever the span", () => {
    // Offering 4.5 to a field the schema types as an integer would produce
    // values the schema then rejects.
    expect(
      resolveBounds(field("integer", { bounds: { minimum: 0, maximum: 3 } }))
        ?.step,
    ).toBe(1);
    expect(
      resolveBounds(field("integer", { bounds: { minimum: 0, maximum: 1 } }))
        ?.step,
    ).toBe(1);
  });

  it("prefers a declared step over a derived one", () => {
    const bounds = resolveBounds(
      field("number", { bounds: { minimum: 0, maximum: 100, step: 25 } }),
    );
    expect(bounds?.step).toBe(25);
  });

  it("ignores a non-positive declared step", () => {
    const bounds = resolveBounds(
      field("integer", { bounds: { minimum: 0, maximum: 10, step: 0 } }),
    );
    expect(bounds?.step).toBe(1);
  });

  it("carries marks through", () => {
    const bounds = resolveBounds(
      field("number", {
        bounds: {
          minimum: 0,
          maximum: 100,
          marks: [{ value: 50, label: "Half" }],
        },
      }),
    );
    expect(bounds?.marks).toEqual([{ value: 50, label: "Half" }]);
  });
});

describe("snapToBounds", () => {
  const bounds = { min: 0, max: 100, step: 5, marks: [] };

  it("clamps outside the track", () => {
    expect(snapToBounds(-20, bounds)).toBe(0);
    expect(snapToBounds(500, bounds)).toBe(100);
  });

  it("snaps to the nearest step", () => {
    expect(snapToBounds(12, bounds)).toBe(10);
    expect(snapToBounds(13, bounds)).toBe(15);
  });

  it("does not accumulate floating-point drift", () => {
    // 0.1 + 3 * 0.1 is 0.30000000000000004, which would show up in the readout.
    const tenths = { min: 0, max: 1, step: 0.1, marks: [] };
    expect(snapToBounds(0.30000000000000004, tenths)).toBe(0.3);
  });
});

describe("trackPercent", () => {
  const bounds = { min: 0, max: 100, step: 1, marks: [] };

  it("maps the ends to 0 and 100", () => {
    expect(trackPercent(0, bounds)).toBe(0);
    expect(trackPercent(100, bounds)).toBe(100);
    expect(trackPercent(50, bounds)).toBe(50);
  });

  it("handles a range that does not start at zero", () => {
    const offset = { min: 10, max: 20, step: 1, marks: [] };
    expect(trackPercent(10, offset)).toBe(0);
    expect(trackPercent(15, offset)).toBe(50);
    expect(trackPercent(20, offset)).toBe(100);
  });
});

describe("the control vocabulary", () => {
  it("maps every control a schema can name to a usable resolution", () => {
    // Guards against a new variant being added to the type and silently
    // resolving to the fallback everywhere.
    const requests: Array<[UiNode["control"], UiNode]> = [
      ["text", field("string", { control: "text" })],
      ["textarea", field("string", { control: "textarea" })],
      ["color", field("string", { control: "color" })],
      ["select", field("string", { control: "select", enumOptions: ["a"] })],
      [
        "segmented",
        field("string", { control: "segmented", enumOptions: ["a"] }),
      ],
      ["radio", field("string", { control: "radio", enumOptions: ["a"] })],
      ["switch", field("boolean", { control: "switch" })],
      ["checkbox", field("boolean", { control: "checkbox" })],
      ["slider", field("number", { control: "slider", bounds: bounded })],
      ["range", pair({ control: "range", bounds: bounded })],
    ];

    for (const [name, node] of requests) {
      expect(resolveControl(node), `control \`${name}\``).toBe(name);
    }
  });
});
