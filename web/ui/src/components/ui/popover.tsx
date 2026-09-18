import * as React from "react";
import { createPortal } from "react-dom";
import { cn } from "@/lib/utils";

/**
 * Popover built on a fixed-position layer rather than a floating-ui dependency.
 *
 * The embedded build ships as one HTML file and the dependency is not
 * installed, so this covers the cases the form actually needs: anchored below
 * (or above) the trigger, dismissed by outside click, Escape, or scroll. It is
 * deliberately not a general-purpose floating layer — there is no collision
 * detection against the viewport edges beyond flipping above when there is no
 * room below, and no arrow.
 */

interface PopoverContextValue {
  open: boolean;
  setOpen(open: boolean): void;
  /** The trigger button, used as the anchor for positioning. */
  triggerRef: React.RefObject<HTMLButtonElement | null>;
  contentId: string;
}

const PopoverContext = React.createContext<PopoverContextValue | null>(null);

function usePopover(): PopoverContextValue {
  const context = React.useContext(PopoverContext);
  if (!context) {
    throw new Error("Popover parts must be used inside <Popover>");
  }
  return context;
}

export function Popover({
  open: controlled,
  onOpenChange,
  children,
}: {
  open?: boolean;
  onOpenChange?(open: boolean): void;
  children: React.ReactNode;
}) {
  const [uncontrolled, setUncontrolled] = React.useState(false);
  const open = controlled ?? uncontrolled;
  const triggerRef = React.useRef<HTMLButtonElement | null>(null);
  const contentId = React.useId();

  const setOpen = React.useCallback(
    (next: boolean) => {
      if (controlled === undefined) setUncontrolled(next);
      onOpenChange?.(next);
    },
    [controlled, onOpenChange],
  );

  return (
    <PopoverContext.Provider value={{ open, setOpen, triggerRef, contentId }}>
      {children}
    </PopoverContext.Provider>
  );
}

/**
 * The trigger renders its own button and is styled through `className`.
 *
 * It deliberately does not clone a caller-supplied element: positioning needs a
 * handle on the trigger node, and handing a ref to a cloned child both trips the
 * hooks lint rule and makes the anchor ambiguous when the child is not a DOM
 * element. The caller passes contents and styling instead.
 */
export function PopoverTrigger({
  children,
  className,
  disabled,
  ...rest
}: React.ComponentPropsWithoutRef<"button">) {
  const { open, setOpen, triggerRef, contentId } = usePopover();

  return (
    <button
      {...rest}
      ref={triggerRef}
      type="button"
      disabled={disabled}
      aria-haspopup="dialog"
      aria-expanded={open}
      aria-controls={open ? contentId : undefined}
      onClick={() => setOpen(!open)}
      className={className}
    >
      {children}
    </button>
  );
}

/** Anchor is a no-op here: positioning always follows the trigger. */
export function PopoverAnchor({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}

export function PopoverClose({
  children,
}: {
  children: React.ReactElement;
}) {
  const { setOpen } = usePopover();
  const child = children as React.ReactElement<{
    onClick?: (event: React.MouseEvent) => void;
  }>;
  return React.cloneElement(child, {
    onClick: (event: React.MouseEvent) => {
      child.props.onClick?.(event);
      setOpen(false);
    },
  });
}

export function PopoverContent({
  className,
  align = "start",
  sideOffset = 6,
  children,
}: {
  className?: string;
  align?: "start" | "center" | "end";
  sideOffset?: number;
  children: React.ReactNode;
}) {
  const { open, setOpen, triggerRef, contentId } = usePopover();
  const contentRef = React.useRef<HTMLDivElement | null>(null);
  const [position, setPosition] = React.useState<
    {
      top: number;
      left: number;
    } | null
  >(null);

  // Re-measure while open: the trigger can move under a scroll or a resize, and
  // a stale anchor would leave the panel floating over unrelated content.
  React.useLayoutEffect(() => {
    if (!open) {
      setPosition(null);
      return;
    }

    const place = () => {
      const trigger = triggerRef.current;
      const content = contentRef.current;
      if (!trigger || !content) return;

      const anchor = trigger.getBoundingClientRect();
      const box = content.getBoundingClientRect();
      const gap = sideOffset;

      const below = anchor.bottom + gap;
      const above = anchor.top - box.height - gap;
      // Flip above only when the panel genuinely does not fit below.
      const top = below + box.height <= window.innerHeight || above < 0
        ? below
        : above;

      let left = align === "end"
        ? anchor.right - box.width
        : align === "center"
        ? anchor.left + (anchor.width - box.width) / 2
        : anchor.left;
      // Keep it on screen: a panel hanging off the right edge is unusable.
      left = Math.min(Math.max(8, left), window.innerWidth - box.width - 8);

      setPosition({ top, left });
    };

    place();
    // Second pass once the panel has its real size, so a first paint that was
    // measured at zero height does not leave it mispositioned.
    const frame = requestAnimationFrame(place);

    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
    };
  }, [open, align, sideOffset, triggerRef]);

  React.useEffect(() => {
    if (!open) return;

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (contentRef.current?.contains(target)) return;
      if (triggerRef.current?.contains(target)) return;
      setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };

    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open, setOpen, triggerRef]);

  if (!open) return null;

  return createPortal(
    <div
      ref={contentRef}
      id={contentId}
      role="dialog"
      style={{
        position: "fixed",
        top: position?.top ?? -9999,
        left: position?.left ?? -9999,
        // Hidden until measured so the first frame never flashes at the wrong
        // spot.
        visibility: position ? "visible" : "hidden",
      }}
      className={cn(
        "z-50 rounded-lg border border-border bg-popover p-3 text-popover-foreground shadow-lg outline-none",
        className,
      )}
    >
      {children}
    </div>,
    document.body,
  );
}
