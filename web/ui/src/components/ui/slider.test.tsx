import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Slider } from "./slider";

/** The range inputs, in DOM order — low handle first. */
function handles(): HTMLInputElement[] {
  return screen.getAllByRole("slider") as HTMLInputElement[];
}

describe("Slider", () => {
  it("renders one handle for a single value", () => {
    render(
      <Slider value={[30]} onValueChange={() => {}} min={0} max={100} />,
    );

    expect(handles()).toHaveLength(1);
    expect(handles()[0]).toHaveValue("30");
  });

  it("reports a new value when the handle moves", () => {
    const onValueChange = vi.fn();
    render(
      <Slider value={[30]} onValueChange={onValueChange} min={0} max={100} />,
    );

    fireEvent.change(handles()[0], { target: { value: "70" } });

    expect(onValueChange).toHaveBeenCalledWith([70]);
  });

  it("renders two handles for a pair and labels which is which", () => {
    render(
      <Slider value={[20, 80]} onValueChange={() => {}} min={0} max={100} />,
    );

    const [low, high] = handles();
    expect(low).toHaveValue("20");
    expect(high).toHaveValue("80");
    // Screen-reader users get the two ends named; sighted users read the
    // readout below the track.
    expect(low).toHaveAccessibleName("Minimum");
    expect(high).toHaveAccessibleName("Maximum");
  });

  it("moves the requested handle, not the other one", () => {
    const onValueChange = vi.fn();
    render(
      <Slider
        value={[20, 80]}
        onValueChange={onValueChange}
        min={0}
        max={100}
      />,
    );

    fireEvent.change(handles()[1], { target: { value: "90" } });

    expect(onValueChange).toHaveBeenCalledWith([20, 90]);
  });

  it("stops the low handle crossing the high one", () => {
    // Native range inputs know nothing about each other, so without this the
    // drag would silently swap which end of the interval is which.
    const onValueChange = vi.fn();
    render(
      <Slider
        value={[20, 80]}
        onValueChange={onValueChange}
        min={0}
        max={100}
        step={1}
      />,
    );

    fireEvent.change(handles()[0], { target: { value: "95" } });

    expect(onValueChange).toHaveBeenCalledWith([79, 80]);
  });

  it("stops the high handle crossing the low one", () => {
    const onValueChange = vi.fn();
    render(
      <Slider
        value={[20, 80]}
        onValueChange={onValueChange}
        min={0}
        max={100}
        step={1}
      />,
    );

    fireEvent.change(handles()[1], { target: { value: "5" } });

    expect(onValueChange).toHaveBeenCalledWith([20, 21]);
  });

  it("keeps a wider gap when asked for one", () => {
    const onValueChange = vi.fn();
    render(
      <Slider
        value={[20, 80]}
        onValueChange={onValueChange}
        min={0}
        max={100}
        step={5}
        minStepsBetweenThumbs={4}
      />,
    );

    fireEvent.change(handles()[0], { target: { value: "100" } });

    // Four steps of five keeps the handles 20 apart.
    expect(onValueChange).toHaveBeenCalledWith([60, 80]);
  });

  it("stops the low handle reaching the far end of the track", () => {
    const onValueChange = vi.fn();
    render(
      <Slider
        value={[0, 100]}
        onValueChange={onValueChange}
        min={0}
        max={100}
        step={1}
      />,
    );

    fireEvent.change(handles()[0], { target: { value: "100" } });

    // One step short of the high handle, which is itself at the end.
    expect(onValueChange).toHaveBeenCalledWith([99, 100]);
  });

  it("lets the browser hold each handle inside the track", () => {
    // A native range input clamps to its own min/max, so an out-of-range drag
    // never reaches the handler. The component does not need to re-clamp, and
    // this records that the guarantee comes from the platform.
    render(
      <Slider value={[0, 100]} onValueChange={() => {}} min={0} max={100} />,
    );

    fireEvent.change(handles()[0], { target: { value: "-5" } });

    expect(handles()[0].value).toBe("0");
  });

  it("shows the value only when asked", () => {
    const { rerender } = render(
      <Slider value={[42]} onValueChange={() => {}} min={0} max={100} />,
    );
    expect(screen.queryByText("42")).not.toBeInTheDocument();

    rerender(
      <Slider
        value={[42]}
        onValueChange={() => {}}
        min={0}
        max={100}
        showValue
      />,
    );
    expect(screen.getByText("42")).toBeInTheDocument();
  });

  it("joins both ends in the readout for a pair", () => {
    render(
      <Slider
        value={[20, 80]}
        onValueChange={() => {}}
        min={0}
        max={100}
        showValue
      />,
    );

    expect(screen.getByText("20 – 80")).toBeInTheDocument();
  });

  it("prints marks at their position, falling back to the value", () => {
    render(
      <Slider
        value={[50]}
        onValueChange={() => {}}
        min={0}
        max={100}
        marks={[
          { value: 0, label: "Off" },
          { value: 50 },
          { value: 100, label: "Max" },
        ]}
      />,
    );

    expect(screen.getByText("Off")).toBeInTheDocument();
    expect(screen.getByText("Max")).toBeInTheDocument();
    // An unlabelled mark still gets a tick with its number.
    expect(screen.getByText("50")).toBeInTheDocument();
  });

  it("honours a custom value formatter", () => {
    render(
      <Slider
        value={[0.5]}
        onValueChange={() => {}}
        min={0}
        max={1}
        step={0.01}
        showValue
        formatValue={(value) => `${Math.round(value * 100)}%`}
      />,
    );

    expect(screen.getByText("50%")).toBeInTheDocument();
  });

  it("disables every handle together", () => {
    render(
      <Slider
        value={[20, 80]}
        onValueChange={() => {}}
        min={0}
        max={100}
        disabled
      />,
    );

    for (const handle of handles()) {
      expect(handle).toBeDisabled();
    }
  });
});
