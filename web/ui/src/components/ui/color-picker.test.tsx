import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ColorPicker } from "./color-picker";

/** The saturation/brightness square, which is hidden from assistive tech. */
function saturationArea(): HTMLElement {
  const area = document.querySelector<HTMLElement>(
    "[aria-hidden='true'].cursor-crosshair",
  );
  if (!area) throw new Error("saturation area should be rendered");
  return area;
}

/** The hue track, also hidden from assistive tech. */
function hueTrack(): HTMLElement {
  const track = document.querySelector<HTMLElement>(
    "[aria-hidden='true'].cursor-pointer",
  );
  if (!track) throw new Error("hue track should be rendered");
  return track;
}

/**
 * The shared test rect is 200x100 at the origin, so a coordinate maps to a
 * fraction directly: `clientX` 100 is halfway across, `clientY` 50 halfway down.
 */
function press(target: HTMLElement, clientX: number, clientY: number) {
  fireEvent.pointerDown(target, { clientX, clientY, pointerId: 1 });
}

describe("ColorPicker", () => {
  it("shows the current colour on the trigger", () => {
    render(<ColorPicker value="#3b82f6" onChange={() => {}} />);

    expect(screen.getByText("#3b82f6")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Colour #3b82f6" }),
    ).toBeInTheDocument();
  });

  it("says so when no colour is set", () => {
    render(<ColorPicker value="" onChange={() => {}} />);

    expect(screen.getByText("—")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Colour unset" }),
    ).toBeInTheDocument();
  });

  it("opens the panel on press and closes it on Escape", async () => {
    const user = userEvent.setup();
    render(<ColorPicker value="#000000" onChange={() => {}} />);

    await user.click(screen.getByRole("button", { name: "Colour #000000" }));
    expect(screen.getByLabelText("Hex colour")).toBeInTheDocument();

    await user.keyboard("{Escape}");
    expect(screen.queryByLabelText("Hex colour")).not.toBeInTheDocument();
  });

  it("accepts a hex typed without the hash", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ColorPicker value="#000000" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Colour #000000" }));
    const input = screen.getByLabelText("Hex colour");
    await user.clear(input);
    await user.type(input, "3b82f6");

    // The value is normalised on the way out, so the field always holds a
    // colour a CSS parser accepts.
    expect(onChange).toHaveBeenLastCalledWith("#3b82f6");
  });

  it("keeps a half-typed hex visible without committing it", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ColorPicker value="#000000" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Colour #000000" }));
    const input = screen.getByLabelText("Hex colour");
    await user.clear(input);
    await user.type(input, "ab");

    expect(onChange).not.toHaveBeenCalled();
    // Clearing the field mid-edit would make it impossible to type into.
    expect(input).toHaveValue("ab");
  });

  it("resets the field to the committed value on blur", async () => {
    const user = userEvent.setup();
    render(<ColorPicker value="#000000" onChange={() => {}} />);

    await user.click(screen.getByRole("button", { name: "Colour #000000" }));
    const input = screen.getByLabelText("Hex colour");
    await user.clear(input);
    await user.type(input, "ab");
    await user.tab();

    // An abandoned edit should not leave a value that was never applied.
    expect(input).toHaveValue("000000");
  });

  it("picks a preset swatch", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ColorPicker value="#000000" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Colour #000000" }));
    await user.click(screen.getByRole("button", { name: "#22c55e" }));

    expect(onChange).toHaveBeenCalledWith("#22c55e");
  });

  it("marks the swatch that matches the current value", async () => {
    const user = userEvent.setup();
    render(<ColorPicker value="#22c55e" onChange={() => {}} />);

    await user.click(screen.getByRole("button", { name: "Colour #22c55e" }));

    // The check mark is the only child of the active swatch, and it is what
    // tells the user which preset they are on.
    const active = screen.getByRole("button", { name: "#22c55e" });
    expect(active.querySelector("svg")).not.toBeNull();
    const inactive = screen.getByRole("button", { name: "#ef4444" });
    expect(inactive.querySelector("svg")).toBeNull();
  });

  it("reads saturation and brightness off a drag in the square", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ColorPicker value="#ff0000" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Colour #ff0000" }));
    press(saturationArea(), 100, 50);

    // Halfway across is 50% saturation; halfway down is 50% brightness.
    expect(onChange).toHaveBeenCalledWith("#804040");
  });

  it("keeps tracking once the pointer leaves the square", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ColorPicker value="#ff0000" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Colour #ff0000" }));
    const area = saturationArea();
    press(area, 100, 50);
    onChange.mockClear();

    // Well outside the 200x100 element. Without pointer capture the drag would
    // stop the moment the cursor left the square, and the colour would freeze
    // at the midpoint; with it, the position clamps to the far corner.
    fireEvent.pointerMove(area, { clientX: 400, clientY: 200, pointerId: 1 });

    expect(onChange).toHaveBeenCalledWith("#000000");
  });

  it("ignores a move when it is not the element holding the drag", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ColorPicker value="#ff0000" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Colour #ff0000" }));
    onChange.mockClear();

    // A hover is not a drag.
    fireEvent.pointerMove(saturationArea(), {
      clientX: 100,
      clientY: 50,
      pointerId: 1,
    });

    expect(onChange).not.toHaveBeenCalled();
  });

  it("clamps a drag that goes past the edge of the square", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ColorPicker value="#ff0000" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Colour #ff0000" }));
    press(saturationArea(), -80, -80);

    // The top-left corner is zero saturation at full brightness, which is
    // white — not a negative fraction that would render as `NaN` in the hex.
    expect(onChange).toHaveBeenCalledWith("#ffffff");
  });

  it("reads hue off a drag along the track", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ColorPicker value="#ff0000" onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Colour #ff0000" }));
    press(hueTrack(), 100, 0);

    // Halfway along a 360° track is 180°, which on a fully saturated, fully
    // bright colour is cyan.
    expect(onChange).toHaveBeenCalledWith("#00ffff");
  });

  it("keeps the hue when saturation is dragged to zero", async () => {
    // A grey has no hue to recover, so the picker has to remember the one the
    // colour had. Starting from green, going grey, then coming back must land
    // on green — not on red, which is where re-deriving the hue from a grey
    // would put it.
    const user = userEvent.setup();
    const onChange = vi.fn();
    const { rerender } = render(
      <ColorPicker value="#00ff00" onChange={onChange} />,
    );

    await user.click(screen.getByRole("button", { name: "Colour #00ff00" }));
    press(saturationArea(), 0, 0);
    expect(onChange).toHaveBeenLastCalledWith("#ffffff");

    // Feed the committed value back, as the real form does.
    rerender(<ColorPicker value="#ffffff" onChange={onChange} />);
    onChange.mockClear();

    press(saturationArea(), 200, 0);

    expect(onChange).toHaveBeenCalledWith("#00ff00");
  });

  it("is disabled as a whole", () => {
    render(<ColorPicker value="#000000" onChange={() => {}} disabled />);

    expect(screen.getByRole("button", { name: "Colour #000000" }))
      .toBeDisabled();
  });
});
