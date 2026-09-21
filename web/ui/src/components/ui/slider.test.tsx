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

  describe("editable readout", () => {
    /** The badge that turns into the input. */
    function badge(name = "value"): HTMLElement {
      return screen.getByRole("button", { name: `Edit ${name}` });
    }

    /** The input the badge turns into. */
    function field(name = "value"): HTMLInputElement {
      return screen.getByRole("textbox", { name: `Edit ${name}` }) as HTMLInputElement;
    }

    function type(input: HTMLInputElement, text: string) {
      fireEvent.change(input, { target: { value: text } });
    }

    it("stays a plain span when neither editable nor stepper is asked for", () => {
      render(
        <Slider value={[42]} onValueChange={() => {}} min={0} max={100} showValue />,
      );

      expect(screen.getByText("42").tagName.toLowerCase()).toBe("span");
      expect(screen.queryByRole("button", { name: /Edit/ })).not.toBeInTheDocument();
    });

    it("opens on click, pre-filled with what the badge was showing", () => {
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

      fireEvent.click(badge());

      expect(field().value).toBe("42");
    });

    it("commits on Enter and closes", () => {
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

      fireEvent.click(badge());
      type(field(), "75");
      fireEvent.keyDown(field(), { key: "Enter" });

      expect(onValueChange).toHaveBeenCalledWith([75]);
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("commits on blur", () => {
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

      fireEvent.click(badge());
      type(field(), "80");
      fireEvent.blur(field());

      expect(onValueChange).toHaveBeenCalledWith([80]);
    });

    it("restores the previous value on Escape", () => {
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

      fireEvent.click(badge());
      type(field(), "99");
      fireEvent.keyDown(field(), { key: "Escape" });

      expect(onValueChange).not.toHaveBeenCalled();
      expect(badge()).toHaveTextContent("42");
    });

    // Focus is the half of "cancel" that is easy to forget: a keyboard user who
    // is dropped on <body> has to tab in from the top of the form again.
    it("hands focus back to the badge after Escape", () => {
      render(
        <Slider
          value={[42]}
          onValueChange={() => {}}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      fireEvent.click(badge());
      expect(document.activeElement).toBe(field());

      fireEvent.keyDown(field(), { key: "Escape" });

      expect(document.activeElement).toBe(badge());
    });

    it("hands focus back to the badge after a commit", () => {
      render(
        <Slider
          value={[42]}
          onValueChange={() => {}}
          min={0}
          max={100}
          step={1}
          showValue
          editable
        />,
      );

      fireEvent.click(badge());
      type(field(), "50");
      fireEvent.keyDown(field(), { key: "Enter" });

      expect(document.activeElement).toBe(badge());
    });

    it("keeps Escape from reaching an enclosing overlay", () => {
      const onOuterEscape = vi.fn();
      render(
        <div
          onKeyDown={(event) => {
            if (event.key === "Escape") onOuterEscape();
          }}
        >
          <Slider
            value={[42]}
            onValueChange={() => {}}
            min={0}
            max={100}
            showValue
            editable
          />
        </div>,
      );

      fireEvent.click(badge());
      fireEvent.keyDown(field(), { key: "Escape" });

      expect(onOuterEscape).not.toHaveBeenCalled();
    });

    it("steps the draft with the arrow keys before committing", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[40]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={5}
          showValue
          editable
        />,
      );

      fireEvent.click(badge());
      fireEvent.keyDown(field(), { key: "ArrowUp" });
      fireEvent.keyDown(field(), { key: "ArrowUp" });

      expect(field().value).toBe("50");
      expect(onValueChange).not.toHaveBeenCalled();

      fireEvent.keyDown(field(), { key: "Enter" });
      expect(onValueChange).toHaveBeenCalledWith([50]);
    });
  });

  describe("readout validation", () => {
    function badge(name = "value"): HTMLElement {
      return screen.getByRole("button", { name: `Edit ${name}` });
    }

    function field(name = "value"): HTMLInputElement {
      return screen.getByRole("textbox", { name: `Edit ${name}` }) as HTMLInputElement;
    }

    function enter(text: string) {
      fireEvent.click(badge());
      fireEvent.change(field(), { target: { value: text } });
      fireEvent.keyDown(field(), { key: "Enter" });
    }

    // A number the track cannot represent still says what the user wanted, so
    // it is moved to the nearest stop rather than thrown away.
    it("clamps a value above the top of the track and says so", () => {
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

      enter("150");

      expect(onValueChange).toHaveBeenCalledWith([100]);
      expect(screen.getByRole("status")).toHaveTextContent("Clamped to 100");
    });

    it("clamps a value below the bottom of the track and says so", () => {
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

      enter("5");

      expect(onValueChange).toHaveBeenCalledWith([10]);
      expect(screen.getByRole("status")).toHaveTextContent("Clamped to 10");
    });

    it("rounds an off-step value to the nearest stop and says so", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[20]}
          onValueChange={onValueChange}
          min={0}
          max={300}
          step={15}
          showValue
          editable
        />,
      );

      enter("100");

      expect(onValueChange).toHaveBeenCalledWith([105]);
      expect(screen.getByRole("status")).toHaveTextContent("Rounded to 105");
    });

    it("says nothing when the value lands exactly", () => {
      render(
        <Slider
          value={[20]}
          onValueChange={() => {}}
          min={0}
          max={100}
          step={10}
          showValue
          editable
        />,
      );

      enter("70");

      expect(screen.getByRole("status")).toHaveTextContent("");
    });

    // Only text with no number in it at all is refused, and the field stays
    // open so the user can correct it.
    it("refuses text that holds no number, naming what it could not read", () => {
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

      enter("not a number");

      expect(onValueChange).not.toHaveBeenCalled();
      expect(field()).toHaveAttribute("aria-invalid", "true");
      expect(screen.getByRole("alert")).toHaveTextContent("not a number");
    });

    it("keeps the current value when unreadable text is blurred away", () => {
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

      fireEvent.click(badge());
      fireEvent.change(field(), { target: { value: "???" } });
      fireEvent.blur(field());

      expect(onValueChange).not.toHaveBeenCalled();
      expect(badge()).toHaveTextContent("42");
      expect(screen.getByRole("status")).toHaveTextContent("Kept 42");
    });

    // `min + n * step` is where 0.30000000000000004 comes from, and a readout
    // that prints that has told the user something untrue about their input.
    it("commits a fractional step without float residue", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[0]}
          onValueChange={onValueChange}
          min={0}
          max={1}
          step={0.1}
          showValue
          editable
        />,
      );

      enter("0.3");

      expect(onValueChange).toHaveBeenCalledWith([0.3]);
    });

    it("keeps float residue out of the message too", () => {
      render(
        <Slider
          value={[0.2, 0.8]}
          onValueChange={() => {}}
          min={0}
          max={1}
          step={0.1}
          showValue
          editable
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit minimum value" }));
      fireEvent.change(
        screen.getByRole("textbox", { name: "Edit minimum value" }),
        { target: { value: "0.95" } },
      );
      fireEvent.keyDown(
        screen.getByRole("textbox", { name: "Edit minimum value" }),
        { key: "Enter" },
      );

      expect(screen.getByRole("status")).toHaveTextContent("Clamped to 0.7");
    });
  });

  describe("readout parsing", () => {
    function badge(name = "value"): HTMLElement {
      return screen.getByRole("button", { name: `Edit ${name}` });
    }

    function field(name = "value"): HTMLInputElement {
      return screen.getByRole("textbox", { name: `Edit ${name}` }) as HTMLInputElement;
    }

    // The motivating case from the issue: the stop is shown as a word, so the
    // word is what the user reaches for.
    it("reads a mark's label back as that mark's value", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[1]}
          onValueChange={onValueChange}
          min={1}
          max={5}
          step={1}
          marks={[
            { value: 1, label: "tiny" },
            { value: 3, label: "medium" },
            { value: 5, label: "huge" },
          ]}
          showValue
          editable
        />,
      );

      fireEvent.click(badge());
      fireEvent.change(field(), { target: { value: "medium" } });
      fireEvent.keyDown(field(), { key: "Enter" });

      expect(onValueChange).toHaveBeenCalledWith([3]);
    });

    it("reads a number wearing a unit", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[0]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={5}
          marks={[{ value: 50, label: "50%" }]}
          showValue
          editable
        />,
      );

      fireEvent.click(badge());
      fireEvent.change(field(), { target: { value: "45%" } });
      fireEvent.keyDown(field(), { key: "Enter" });

      expect(onValueChange).toHaveBeenCalledWith([45]);
    });

    // `formatValue` on its own used to produce a draft the default parser could
    // not read, so opening the field and pressing Enter reported an error on a
    // value the user had not touched.
    it("reads back its own formatted output without a matching parser", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[50]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={1}
          showValue
          editable
          formatValue={(current) => `${current}%`}
        />,
      );

      fireEvent.click(badge());
      fireEvent.keyDown(field(), { key: "Enter" });

      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("honours a custom parser", () => {
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
          formatValue={(current) => `f/${current}`}
          parseValue={(text) => {
            const clean = text.replace(/^f\//i, "").trim();
            const num = Number(clean);
            return Number.isFinite(num) ? num : null;
          }}
        />,
      );

      expect(badge()).toHaveTextContent("f/2.8");

      fireEvent.click(badge());
      fireEvent.change(field(), { target: { value: "f/4.0" } });
      fireEvent.keyDown(field(), { key: "Enter" });

      expect(onValueChange).toHaveBeenCalledWith([4]);
    });
  });

  describe("readout naming", () => {
    it("names the quantity when told what it is", () => {
      render(
        <Slider
          value={[50]}
          onValueChange={() => {}}
          min={0}
          max={100}
          showValue
          editable
          valueLabel="Opacity"
        />,
      );

      expect(screen.getByRole("button", { name: "Edit Opacity" })).toBeInTheDocument();
    });

    it("names both ends of a range", () => {
      render(
        <Slider
          value={[20, 80]}
          onValueChange={() => {}}
          min={0}
          max={100}
          showValue
          editable
          valueLabel="Bandwidth"
        />,
      );

      expect(screen.getByRole("button", { name: "Edit Bandwidth minimum" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Edit Bandwidth maximum" })).toBeInTheDocument();
    });
  });

  describe("readout on a range", () => {
    it("edits each end on its own", () => {
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

      fireEvent.click(screen.getByRole("button", { name: "Edit minimum value" }));
      fireEvent.change(
        screen.getByRole("textbox", { name: "Edit minimum value" }),
        { target: { value: "35" } },
      );
      fireEvent.keyDown(
        screen.getByRole("textbox", { name: "Edit minimum value" }),
        { key: "Enter" },
      );

      expect(onValueChange).toHaveBeenCalledWith([35, 80]);
    });

    it("holds the gap between the handles rather than refusing the edit", () => {
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

      // The gap is 5 * 2, so the low handle stops at 70.
      fireEvent.click(screen.getByRole("button", { name: "Edit minimum value" }));
      fireEvent.change(
        screen.getByRole("textbox", { name: "Edit minimum value" }),
        { target: { value: "75" } },
      );
      fireEvent.keyDown(
        screen.getByRole("textbox", { name: "Edit minimum value" }),
        { key: "Enter" },
      );

      expect(onValueChange).toHaveBeenCalledWith([70, 80]);
      expect(screen.getByRole("status")).toHaveTextContent("Clamped to 70");
    });
  });

  describe("stepper", () => {
    it("never shows while idle, on a single value or a range", () => {
      render(
        <Slider
          value={[40]}
          onValueChange={() => {}}
          min={0}
          max={100}
          step={5}
          showValue
          editable
          stepper
        />,
      );
      expect(screen.queryByRole("button", { name: "Increase value" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Decrease value" })).not.toBeInTheDocument();
    });

    it("moves one step per press once editing", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[40]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={5}
          showValue
          editable
          stepper
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      fireEvent.click(screen.getByRole("button", { name: "Increase value" }));
      const input = screen.getByRole("textbox", { name: "Edit value" }) as HTMLInputElement;
      expect(input.value).toBe("45");

      fireEvent.keyDown(input, { key: "Enter" });
      expect(onValueChange).toHaveBeenCalledWith([45]);
    });

    it("stops at the ends of the track", () => {
      render(
        <Slider
          value={[100]}
          onValueChange={() => {}}
          min={0}
          max={100}
          step={5}
          showValue
          editable
          stepper
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit value" }));
      expect(screen.getByRole("button", { name: "Increase value" })).toBeDisabled();
      expect(screen.getByRole("button", { name: "Decrease value" })).not.toBeDisabled();
    });

    it("stops at the other handle on a range when edited", () => {
      render(
        <Slider
          value={[75, 80]}
          onValueChange={() => {}}
          min={0}
          max={100}
          step={5}
          showValue
          editable
          stepper
        />,
      );

      // On range, idle readouts do not crowd with 4 stepper buttons
      expect(screen.queryByRole("button", { name: "Increase minimum value" })).not.toBeInTheDocument();

      // Entering edit mode on the minimum value displays stepper buttons and respects the boundary
      fireEvent.click(screen.getByRole("button", { name: "Edit minimum value" }));
      expect(screen.getByRole("button", { name: "Increase minimum value" })).toBeDisabled();
      expect(screen.getByRole("button", { name: "Decrease minimum value" })).not.toBeDisabled();
    });

    it("steps the draft using edit-mode stepper buttons and commits on Enter", () => {
      const onValueChange = vi.fn();
      render(
        <Slider
          value={[20, 80]}
          onValueChange={onValueChange}
          min={0}
          max={100}
          step={5}
          showValue
          editable
          stepper
        />,
      );

      fireEvent.click(screen.getByRole("button", { name: "Edit minimum value" }));
      const plus = screen.getByRole("button", { name: "Increase minimum value" });
      fireEvent.click(plus);

      const input = screen.getByRole("textbox", { name: "Edit minimum value" }) as HTMLInputElement;
      expect(input.value).toBe("25");
      expect(onValueChange).not.toHaveBeenCalled();

      fireEvent.keyDown(input, { key: "Enter" });
      expect(onValueChange).toHaveBeenCalledWith([25, 80]);
    });
  });

  describe("readout when input is refused", () => {
    it("disabled leaves the badge unusable", () => {
      render(
        <Slider
          value={[42]}
          onValueChange={() => {}}
          min={0}
          max={100}
          showValue
          editable
          stepper
          disabled
        />,
      );

      const trigger = screen.getByRole("button", { name: "value" });
      expect(trigger).toBeDisabled();
      // Disabled means editing never opens, so the idle stepper (which only
      // ever shows once editing) never appears either.
      expect(screen.queryByRole("button", { name: "Increase value" })).not.toBeInTheDocument();
      fireEvent.click(trigger);
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("readOnly leaves the badge and every handle unusable", () => {
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

      const trigger = screen.getByRole("button", { name: "value" });
      expect(trigger).toBeDisabled();
      fireEvent.click(trigger);
      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();

      for (const handle of handles()) {
        expect(handle).toBeDisabled();
      }
    });
  });
});
