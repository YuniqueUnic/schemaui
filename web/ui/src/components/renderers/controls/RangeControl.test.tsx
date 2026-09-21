import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import type { JsonValue, UiNode } from "../../../types";
import { RangeControl } from "./RangeControl";

/** A two-number array node with both ends of the interval declared. */
function pairNode(
  bounds: Partial<NonNullable<UiNode["bounds"]>> = {},
): UiNode {
  return {
    pointer: "/window",
    required: false,
    control: "range",
    bounds: {
      minimum: 0,
      maximum: 100,
      step: null,
      marks: [],
      ...bounds,
    },
    kind: {
      type: "array",
      item: { type: "field", scalar: "number" },
      min_items: 2,
      max_items: 2,
    },
  };
}

/** The two handles, low first. */
function handles(): HTMLInputElement[] {
  return [
    screen.getByLabelText("Minimum") as HTMLInputElement,
    screen.getByLabelText("Maximum") as HTMLInputElement,
  ];
}

describe("RangeControl", () => {
  it("renders one track with two handles, not a list editor", () => {
    render(
      <RangeControl
        node={pairNode()}
        value={[20, 80]}
        onChange={() => {}}
      />,
    );

    const [low, high] = handles();
    expect(low.value).toBe("20");
    expect(high.value).toBe("80");
    // A list would offer an "add item" affordance, which would let the user
    // build a three-element array the schema rejects. The buttons that are
    // here belong to the readout, and none of them change the array's length.
    expect(screen.queryByRole("button", { name: /add|remove|delete/i })).toBeNull();
  });

  it("prints each end as its own readout", () => {
    render(
      <RangeControl node={pairNode()} value={[20, 80]} onChange={() => {}} />,
    );

    // Two badges rather than one `20 – 80` string: each end is typed into
    // separately, and a single readout has nowhere to put the caret.
    expect(screen.getByRole("button", { name: "Edit minimum value" }))
      .toHaveTextContent("20");
    expect(screen.getByRole("button", { name: "Edit maximum value" }))
      .toHaveTextContent("80");
    // The ends belong to the track, not to a second row of numbers; the marks
    // row is the only other place a number may appear, and there are none here.
    expect(screen.getAllByText("0")).toHaveLength(1);
    expect(screen.getAllByText("100")).toHaveLength(1);
  });

  it("does not label the track ends twice when marks already do", () => {
    render(
      <RangeControl
        node={pairNode({
          marks: [
            { value: 0, label: "off" },
            { value: 100, label: "full" },
          ],
        })}
        // Away from the marks, so that finding "0" below can only mean the
        // footer row came back — not that a readout happens to hold that value.
        value={[25, 75]}
        onChange={() => {}}
      />,
    );

    expect(screen.getByText("off")).toBeTruthy();
    expect(screen.getByText("full")).toBeTruthy();
    // The bare numbers would sit on the row directly under the marks, saying
    // the same thing in a second notation.
    expect(screen.queryByText("0")).toBeNull();
    expect(screen.queryByText("100")).toBeNull();
  });

  it("sorts and snaps a pair that arrives out of order", () => {
    const onChange = vi.fn();
    render(
      <RangeControl node={pairNode()} value={[20, 80]} onChange={onChange} />,
    );

    // Dragging the low handle past the high one: the slider holds a one-step
    // gap, so the value that reaches `commit` is already bounded — the sort is
    // what guarantees the stored pair still reads low-to-high.
    fireEvent.change(handles()[0], { target: { value: "60" } });

    const [, pair] = onChange.mock.calls[0];
    expect(pair[0]).toBeLessThanOrEqual(pair[1]);
  });

  it("snaps a value that does not land on a step", () => {
    const onChange = vi.fn();
    render(
      <RangeControl
        node={pairNode({ step: 25 })}
        value={[25, 75]}
        onChange={onChange}
      />,
    );

    fireEvent.change(handles()[1], { target: { value: "63" } });

    // 63 is not a multiple of 25; the committed value must be, or the handle
    // would render at a position the schema does not allow.
    expect(onChange).toHaveBeenCalledWith("/window", [25, 75]);
  });

  it("falls back to the full interval when the document holds garbage", () => {
    render(
      <RangeControl
        node={pairNode()}
        value={"nonsense" as JsonValue}
        onChange={() => {}}
      />,
    );

    // `NaN – NaN` would tell the user nothing about what to fix; the schema's
    // own interval is always a valid thing to show.
    expect(screen.getByRole("button", { name: "Edit minimum value" }))
      .toHaveTextContent("0");
    expect(screen.getByRole("button", { name: "Edit maximum value" }))
      .toHaveTextContent("100");
  });

  it("falls back when the stored pair has the wrong length", () => {
    render(
      <RangeControl
        node={pairNode()}
        value={[10] as JsonValue}
        onChange={() => {}}
      />,
    );

    expect(screen.getByRole("button", { name: "Edit minimum value" }))
      .toHaveTextContent("0");
    expect(screen.getByRole("button", { name: "Edit maximum value" }))
      .toHaveTextContent("100");
  });

  it("renders nothing when the schema declares only one end", () => {
    const { container } = render(
      <RangeControl
        node={pairNode({ maximum: undefined })}
        value={[0, 50]}
        onChange={() => {}}
      />,
    );

    // A half-open interval has no track to draw, and `resolveControl` refuses
    // to offer this control in the first place — so reaching here at all means
    // the node changed shape under us, and silence beats a broken track.
    expect(container.textContent).toBe("");
  });
});
