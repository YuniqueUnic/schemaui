import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";

function Harness(
  { onOpenChange }: { onOpenChange?(open: boolean): void } = {},
) {
  return (
    <div>
      <button type="button">outside</button>
      <Popover onOpenChange={onOpenChange}>
        <PopoverTrigger>Open</PopoverTrigger>
        <PopoverContent>
          <button type="button">inside</button>
        </PopoverContent>
      </Popover>
    </div>
  );
}

describe("Popover", () => {
  it("is closed until the trigger is pressed", () => {
    render(<Harness />);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("opens on the trigger and reports the state to the caller", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(<Harness onOpenChange={onOpenChange} />);

    await user.click(screen.getByRole("button", { name: "Open" }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(onOpenChange).toHaveBeenCalledWith(true);
  });

  it("marks the trigger as an expanded popup for assistive tech", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const trigger = screen.getByRole("button", { name: "Open" });

    expect(trigger).toHaveAttribute("aria-expanded", "false");

    await user.click(trigger);

    expect(trigger).toHaveAttribute("aria-expanded", "true");
    // The panel it controls is the one that just appeared.
    expect(trigger).toHaveAttribute(
      "aria-controls",
      screen.getByRole("dialog").id,
    );
  });

  it("closes when the trigger is pressed again", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const trigger = screen.getByRole("button", { name: "Open" });

    await user.click(trigger);
    await user.click(trigger);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("closes on Escape", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    await user.click(screen.getByRole("button", { name: "Open" }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("closes when something outside is pressed", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    await user.click(screen.getByRole("button", { name: "Open" }));
    await user.click(screen.getByRole("button", { name: "outside" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("stays open when something inside is pressed", async () => {
    // A colour picker holds its own controls; dismissing on every press would
    // make it unusable.
    const user = userEvent.setup();
    render(<Harness />);

    await user.click(screen.getByRole("button", { name: "Open" }));
    await user.click(screen.getByRole("button", { name: "inside" }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("renders the panel outside the trigger's own subtree", async () => {
    // It is portalled to the body so a panel cannot be clipped by an ancestor
    // with `overflow: hidden` — which the editor column has.
    const user = userEvent.setup();
    render(<Harness />);

    await user.click(screen.getByRole("button", { name: "Open" }));

    const dialog = screen.getByRole("dialog");
    expect(dialog.parentElement).toBe(document.body);
  });

  it("positions the panel against the trigger rather than the viewport corner", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    await user.click(screen.getByRole("button", { name: "Open" }));

    const dialog = screen.getByRole("dialog");
    // The shared test rect is 200x100 at the origin, so the panel lands just
    // under it and is nudged in from the left edge.
    expect(dialog).toHaveStyle({ position: "fixed", visibility: "visible" });
    expect(dialog.style.top).toBe("106px");
    expect(dialog.style.left).toBe("8px");
  });

  it("keeps the panel on screen when the trigger is near the right edge", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    // A trigger at x=1000 with a 200px-wide panel in a 1024px viewport would
    // hang off the edge if it were anchored naively.
    screen.getByRole("button", { name: "Open" }).getBoundingClientRect = () =>
      ({
        x: 1000,
        y: 0,
        top: 0,
        left: 1000,
        right: 1060,
        bottom: 40,
        width: 60,
        height: 40,
        toJSON: () => ({}),
      }) as DOMRect;

    await user.click(screen.getByRole("button", { name: "Open" }));

    const dialog = screen.getByRole("dialog");
    expect(Number.parseFloat(dialog.style.left)).toBeLessThanOrEqual(
      window.innerWidth - 200,
    );
  });

  it("does not render the panel at all while closed", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    await user.click(screen.getByRole("button", { name: "Open" }));
    await user.click(screen.getByRole("button", { name: "outside" }));

    // Not merely hidden: nothing is left in the DOM to be focused or read.
    expect(document.body.textContent).not.toContain("inside");
  });
});
