import type { JsonValue, UiNode, UiNodeKind } from "../../types";
import { deepEqual } from "../../utils/deepEqual";
import type { EnumFieldKind } from "../../utils/typeHelpers";
import { Checkbox } from "../ui/checkbox";

/**
 * Multi select renderer.
 *
 * An array of enum options is a *set of choices*, so it renders as one control
 * with a checkbox per option instead of a list of independent single-selects.
 * The selection is derived from the value, which makes duplicate entries
 * impossible by construction: `uniqueItems` is satisfied structurally rather
 * than caught by validation after the fact.
 */

type ArrayNode = UiNode & {
  kind: Extract<UiNodeKind, { type: "array" }>;
};

interface MultiSelectRendererProps {
  node: ArrayNode;
  itemKind: EnumFieldKind;
  value: JsonValue | undefined;
  onChange: (pointer: string, value: JsonValue) => void;
}

export function MultiSelectRenderer({
  node,
  itemKind,
  value,
  onChange,
}: MultiSelectRendererProps) {
  const labels = itemKind.enum_options;
  const options = (itemKind.enum_values ?? itemKind.enum_options) as JsonValue[];
  const selected = Array.isArray(value) ? (value as JsonValue[]) : [];

  const isSelected = (option: JsonValue) =>
    selected.some((entry) => deepEqual(entry, option));

  // Rebuilt from `options` on every toggle so the emitted array always follows
  // schema order, whatever order the user happened to click in.
  const toggle = (index: number) => {
    onChange(
      node.pointer,
      options.filter((option, idx) =>
        idx === index ? !isSelected(option) : isSelected(option)
      ),
    );
  };

  return (
    <div className="overflow-hidden rounded-lg border border-border">
      {options.map((option, index) => (
        <label
          key={index}
          className="flex cursor-pointer items-start gap-2.5 border-b border-border px-3 py-2 transition-colors last:border-b-0 hover:bg-muted/60"
        >
          <Checkbox
            className="mt-[3px]"
            checked={isSelected(option)}
            onCheckedChange={() => toggle(index)}
          />
          <span className="text-sm leading-snug">
            {labels[index] ?? String(option)}
          </span>
        </label>
      ))}
    </div>
  );
}
