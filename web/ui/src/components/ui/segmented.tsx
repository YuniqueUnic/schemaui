import * as RadioGroupPrimitive from "@radix-ui/react-radio-group";
import { cn } from "@/lib/utils";

export interface SegmentedOption {
  /** The value handed back on selection. */
  value: string;
  label: string;
}

interface SegmentedProps {
  value: string;
  onValueChange(value: string): void;
  options: SegmentedOption[];
  id?: string;
  disabled?: boolean;
  className?: string;
}

/**
 * Segmented control: every option visible, one selectable.
 *
 * Built on the radio group rather than a row of buttons so the keyboard and
 * screen-reader behaviour comes for free — arrow keys move the selection and
 * the group announces itself as a single choice, which a row of `aria-pressed`
 * buttons does not.
 *
 * Suits short enums. Past roughly five options the row stops fitting and the
 * schema should ask for `select` instead; wrapping is a fallback, not a plan.
 */
export function Segmented({
  value,
  onValueChange,
  options,
  id,
  disabled,
  className,
}: SegmentedProps) {
  return (
    <RadioGroupPrimitive.Root
      id={id}
      value={value}
      onValueChange={onValueChange}
      disabled={disabled}
      className={cn(
        "inline-flex flex-wrap items-center gap-0.5 rounded-lg border border-input bg-muted/60 p-0.5",
        className,
      )}
    >
      {options.map((option) => {
        const active = option.value === value;
        return (
          <RadioGroupPrimitive.Item
            key={option.value}
            value={option.value}
            className={cn(
              "inline-flex items-center justify-center rounded-md px-3 py-1 text-xs font-medium transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background",
              active
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {option.label}
          </RadioGroupPrimitive.Item>
        );
      })}
    </RadioGroupPrimitive.Root>
  );
}
