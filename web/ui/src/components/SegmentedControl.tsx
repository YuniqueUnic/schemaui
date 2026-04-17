import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface SegmentOption<T extends string> {
  id: T;
  label: ReactNode;
  icon?: ReactNode;
}

interface SegmentedControlProps<T extends string> {
  value: T;
  onChange(value: T): void;
  options: SegmentOption<T>[];
  size?: "sm" | "md";
  className?: string;
}

/**
 * SegmentedControl — a compact pill-group used for nav mode toggle,
 * mobile panel switch, and any other "one-of-N" selector.
 */
export function SegmentedControl<T extends string>({
  value,
  onChange,
  options,
  size = "sm",
  className,
}: SegmentedControlProps<T>) {
  const isMd = size === "md";
  return (
    <div
      role="tablist"
      className={cn(
        "inline-flex items-center rounded-full bg-muted/70 p-0.5 backdrop-blur",
        "shadow-[inset_0_0_0_1px_color-mix(in_oklch,var(--color-border)_60%,transparent)]",
        className,
      )}
    >
      {options.map((opt) => {
        const active = value === opt.id;
        return (
          <button
            key={opt.id}
            type="button"
            role="tab"
            aria-selected={active}
            onClick={() => onChange(opt.id)}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-full transition-all",
              isMd ? "px-3 py-1 text-xs" : "px-2.5 py-0.5 text-[11px]",
              "font-medium",
              active
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {opt.icon && (
              <span className="flex items-center">{opt.icon}</span>
            )}
            <span>{opt.label}</span>
          </button>
        );
      })}
    </div>
  );
}
