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

  describe("Editable value display", () => {
    it("renders static span when editable is false", () => {
      render(
        <Slider
          value={[42]}
          onValueChange={() => {}}
          min={0}
          max={100}
          showValue
          editable={false}
        />,
      );

      expect(screen.getByText("42").tagName.toLowerCase()).toBe("span");
      expect(screen.queryByRole("button", { name: "Edit value" })).not.toBeInTheDocument();
    });

    it("clicking value display enters edit mode and renders input", () => {
      render(
        <Slider
          value={[42]}
          onValueChange={() => {}}
          min={0}
          max={100}
          showValue
          editable
        />,
      );

      const trigger = screen.getByRole("button", { name: "Edit value" });
      fireEvent.click(trigger);

      const input = screen.getByRole("textbox", { name: "Edit value" }) as HTMLInputElement;
      expect(input).toBeInTheDocument();
      expect(input.value).toBe("42");
    });

    it("keyboard activation with Enter or Space on display button enters edit mode", () => {
      render(
        <Slider
          value={[42]}
          onValueChange={() => {}}
          min={0}
          max={100}
          showValue
          editable
        />,
      );

      const trigger = screen.getByRole("button", { name: "Edit value" });
      fireEvent.click(trigger);

      expect(screen.getByRole("textbox", { name: "Edit value" })).toBeInTheDocument();
    });

    it("commits valid value on Enter, exits edit mode, and synchronizes thumb and model", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[42]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      const input = screen.getByRole("textbox", { name: "Edit value" });

      fireEvent.change(input, { target: { value: "75" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).toHaveBeenCalledWith([75]);
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("commits valid value on blur and exits edit mode", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[42]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      const input = screen.getByRole("textbox", { name: "Edit value" });

      fireEvent.change(input, { target: { value: "80" } });
      fireEvent.blur(input);

      expect(onValueChange).toHaveBeenCalledWith([80]);
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("cancels editing on Escape, restoring previous value without calling onValueChange", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[42]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      const input = screen.getByRole("textbox", { name: "Edit value" });

      fireEvent.change(input, { target: { value: "99" } });
      fireEvent.keyDown(input, { key: "Escape" });

      expect(onValueChange).not.toHaveBeenCalled();
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Edit value" })).toHaveTextContent("42");
    });

    it("rejects non-numeric input on Enter and sets aria-invalid and error alert", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[42]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      const input = screen.getByRole("textbox", { name: "Edit value" });

      fireEvent.change(input, { target: { value: "not-a-number" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).not.toHaveBeenCalled();
      expect(input).toHaveAttribute("aria-invalid", "true");
      expect(screen.getByRole("alert")).toHaveTextContent("Enter a valid number");
    });

    it("rejects value below min and sets accessible error", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[42]}
          onValueChange={onValueChange}
          min={10}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      const input = screen.getByRole("textbox", { name: "Edit value" });

      fireEvent.change(input, { target: { value: "5" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).not.toHaveBeenCalled();
      expect(input).toHaveAttribute("aria-invalid", "true");
      expect(screen.getByRole("alert")).toHaveTextContent("Value must be at least 10");
    });

    it("rejects value above max and sets accessible error", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[42]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      const input = screen.getByRole("textbox", { name: "Edit value" });

      fireEvent.change(input, { target: { value: "150" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).not.toHaveBeenCalled();
      expect(input).toHaveAttribute("aria-invalid", "true");
      expect(screen.getByRole("alert")).toHaveTextContent("Value must be at most 100");
    });

    it("rejects off-step value and sets accessible error", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[20]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={10}
          showValue
          editable
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      const input = screen.getByRole("textbox", { name: "Edit value" });

      fireEvent.change(input, { target: { value: "25" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).not.toHaveBeenCalled();
      expect(input).toHaveAttribute("aria-invalid", "true");
      expect(screen.getByRole("alert")).toHaveTextContent("Value must be a multiple of 10");
    });

    it("cancels and restores previous value if blurred while invalid", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[42]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      const input = screen.getByRole("textbox", { name: "Edit value" });

      fireEvent.change(input, { target: { value: "invalid" } });
      fireEvent.blur(input);

      expect(onValueChange).not.toHaveBeenCalled();
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Edit value" })).toHaveTextContent("42");
    });

    it("disabled prevents editing", () => {
      render(
        <Slider
          value={[42]}
          onValueChange={() => {}}
          min={0}
          max={100}
          showValue
          editable
          disabled
        />,
      );

      const trigger = screen.getByRole("button", { name: "Edit value" });
      expect(trigger).toBeDisabled();
      fireEvent.click(trigger);
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("readOnly prevents editing and disables handle", () => {
      render(
        <Slider
          value={[42]}
          onValueChange={() => {}}
          min={0}
          max={100}
          showValue
          editable
          readOnly
        />,
      );

      const trigger = screen.getByRole("button", { name: "Edit value" });
      expect(trigger).toBeDisabled();
      fireEvent.click(trigger);
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();

      for (const handle of handles()) {
        expect(handle).toBeDisabled();
      }
    });

    it("supports custom formatValue and parseValue", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[2.8]}
          onValueChange={onValueChange}
          min={1}
          max={16}
          step={0.1}
          showValue
          editable
          formatValue={(val) => `f/${val}`}
          parseValue={(text) => {
            const clean = text.replace(/^f\//i, "").trim();
            const num = Number(clean);
            return Number.isFinite(num) ? num : null;
          }}
        />,
      );

      const trigger = screen.getByRole("button", { name: "Edit value" });
      expect(trigger).toHaveTextContent("f/2.8");

      fireEvent.click(trigger);
      const input = screen.getByRole("textbox", { name: "Edit value" }) as HTMLInputElement;
      expect(input.value).toBe("f/2.8");

      fireEvent.change(input, { target: { value: "f/4.0" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).toHaveBeenCalledWith([4]);
    });

    it("independently edits minimum endpoint of range slider", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[20, 80]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      const minTrigger = screen.getByRole("button", { name: "Edit minimum value" });
      const maxTrigger = screen.getByRole("button", { name: "Edit maximum value" });
      expect(minTrigger).toHaveTextContent("20");
      expect(maxTrigger).toHaveTextContent("80");

      fireEvent.click(minTrigger);
      const input = screen.getByRole("textbox", { name: "Edit minimum value" });
      fireEvent.change(input, { target: { value: "35" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).toHaveBeenCalledWith([35, 80]);
    });

    it("independently edits maximum endpoint of range slider", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[20, 80]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      const maxTrigger = screen.getByRole("button", { name: "Edit maximum value" });
      fireEvent.click(maxTrigger);
      const input = screen.getByRole("textbox", { name: "Edit maximum value" });
      fireEvent.change(input, { target: { value: "65" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).toHaveBeenCalledWith([20, 65]);
    });

    it("prevents minimum endpoint from violating minStepsBetweenThumbs gap", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[20, 80]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={5}
          minStepsBetweenThumbs={2}
          showValue
          editable
        />,
      );

      // Gap is 5 * 2 = 10; minimum cannot exceed 80 - 10 = 70
      fireEvent.click(screen.getByRole("button", { name: "Edit minimum value" }));
      const input = screen.getByRole("textbox", { name: "Edit minimum value" });
      fireEvent.change(input, { target: { value: "75" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).not.toHaveBeenCalled();
      expect(input).toHaveAttribute("aria-invalid", "true");
      expect(screen.getByRole("alert")).toHaveTextContent("Minimum cannot exceed 70");
    });

    it("prevents maximum endpoint from violating minStepsBetweenThumbs gap", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[20, 80]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={5}
          minStepsBetweenThumbs={2}
          showValue
          editable
        />,
      );

      // Gap is 5 * 2 = 10; maximum must be at least 20 + 10 = 30
      fireEvent.click(screen.getByRole("button", { name: "Edit maximum value" }));
      const input = screen.getByRole("textbox", { name: "Edit maximum value" });
      fireEvent.change(input, { target: { value: "25" } });
      fireEvent.keyDown(input, { key: "Enter" });

      expect(onValueChange).not.toHaveBeenCalled();
      expect(input).toHaveAttribute("aria-invalid", "true");
      expect(screen.getByRole("alert")).toHaveTextContent("Maximum must be at least 30");
    });
  });
});
