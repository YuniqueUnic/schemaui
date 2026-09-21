/* eslint-disable react-refresh/only-export-components */

import type { JsonValue, UiNode } from "../../types";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Switch } from "../ui/switch";
import { Checkbox } from "../ui/checkbox";
import { Slider } from "../ui/slider";
import { Segmented } from "../ui/segmented";
import { RadioGroup, RadioGroupItem } from "../ui/radio-group";
import { ColorPicker } from "../ui/color-picker";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import { defaultForKind } from "../../ui-ast";
import { Textarea } from "../ui/textarea";
import type { InlineFieldKind } from "../../utils/typeHelpers";
import { resolveBounds, resolveControl, snapToBounds } from "../../lib/control";

type FieldNode = UiNode & {
  kind: Extract<import("../../types").UiNodeKind, { type: "field" }>;
};

interface FieldRendererProps {
  node: FieldNode;
  value: JsonValue | undefined;
  onChange: (pointer: string, value: JsonValue) => void;
}

function sameJsonValue(left: JsonValue | undefined, right: JsonValue | undefined) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function cloneJsonValue<T extends JsonValue>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/**
 * Renders the control the schema asked for, or the one that suits the value.
 *
 * The decision itself lives in `resolveControl` so that this component only
 * maps a decision to a widget — and so that a control the schema names but this
 * build cannot draw still produces a working field rather than an empty one.
 */
export function FieldRenderer({ node, value, onChange }: FieldRendererProps) {
  const resolved = value === undefined
    ? (node.default_value ?? defaultForKind(node.kind))
    : value;
  const nullable = node.kind.nullable === true;
  const control = resolveControl(node);

  // Enums first: every enum control needs the option list, and the plain
  // scalars below can never be one.
  if (node.kind.enum_options?.length) {
    return (
      <EnumControl
        node={node}
        control={control}
        value={resolved}
        onChange={onChange}
      />
    );
  }

  switch (control) {
    case "color": {
      const text = typeof resolved === "string" ? resolved : "";
      return (
        <ColorPicker
          value={text}
          onChange={(next) => onChange(node.pointer, next)}
        />
      );
    }

    case "slider": {
      const bounds = resolveBounds(node);
      // `resolveControl` only returns `slider` when bounds exist, so this is
      // unreachable in practice; falling through to the number box keeps the
      // component honest if that ever stops being true.
      if (!bounds) break;
      const current = typeof resolved === "number" ? resolved : bounds.min;
      return (
        <Slider
          min={bounds.min}
          max={bounds.max}
          step={bounds.step}
          marks={bounds.marks}
          showValue
          // The readout is the only place an exact value can be given: a drag
          // resolves to whatever pixel the pointer landed on, and arrow keys
          // take one step per press, which is a hundred presses across a
          // fractional track.
          editable
          stepper
          valueLabel={node.title ?? undefined}
          value={[current]}
          onValueChange={([next]) =>
            onChange(node.pointer, snapToBounds(next, bounds))}
        />
      );
    }

    case "switch": {
      const on = resolved === true;
      return (
        <div className="flex items-center gap-3">
          <Switch
            id={controlId(node.pointer)}
            checked={on}
            onCheckedChange={(checked) => onChange(node.pointer, checked)}
          />
          <Label
            htmlFor={controlId(node.pointer)}
            className="text-sm text-muted-foreground"
          >
            {on ? "On" : "Off"}
          </Label>
        </div>
      );
    }

    case "checkbox": {
      const checked = resolved === true;
      return (
        <div className="flex items-center gap-2.5">
          <Checkbox
            id={controlId(node.pointer)}
            checked={checked}
            onCheckedChange={(next) => onChange(node.pointer, next === true)}
          />
          <Label
            htmlFor={controlId(node.pointer)}
            className="text-sm text-muted-foreground"
          >
            {checked ? "Enabled" : "Disabled"}
          </Label>
        </div>
      );
    }

    case "textarea": {
      const text = (resolved as string) ?? "";
      return (
        <Textarea
          rows={4}
          value={text}
          onChange={(event) => commitText(node.pointer, event.target.value, nullable, onChange)}
        />
      );
    }

    case "number":
    case "text":
    default:
      break;
  }

  switch (node.kind.scalar) {
    case "integer":
    case "number":
      return (
        <Input
          type="number"
          value={typeof resolved === "number" ? resolved : ""}
          onChange={(event) => {
            if (event.target.value === "" && nullable) {
              onChange(node.pointer, null);
              return;
            }
            const numValue = Number(event.target.value);
            if (!isNaN(numValue)) {
              onChange(node.pointer, numValue);
            }
          }}
          onBlur={(event) => {
            if (event.target.value === "" && nullable) {
              onChange(node.pointer, null);
              return;
            }
            const numValue = event.target.value === ""
              ? 0
              : Number(event.target.value);
            if (!isNaN(numValue)) {
              onChange(node.pointer, numValue);
            }
          }}
        />
      );
    case "boolean":
      // Reached only when the schema asked for a control that does not fit a
      // boolean and the fallback was overridden; a switch is the safe default.
      return (
        <Switch
          checked={Boolean(resolved)}
          onCheckedChange={(checked) => onChange(node.pointer, checked)}
        />
      );
    case "string":
    default: {
      const text = (resolved as string) ?? "";
      return (
        <Input
          type="text"
          value={text}
          onChange={(event) => commitText(node.pointer, event.target.value, nullable, onChange)}
        />
      );
    }
  }
}

/** The three ways an enum can be presented. */
function EnumControl({
  node,
  control,
  value,
  onChange,
}: {
  node: FieldNode;
  control: ReturnType<typeof resolveControl>;
  value: JsonValue | undefined;
  onChange: (pointer: string, value: JsonValue) => void;
}) {
  const labels = node.kind.enum_options ?? [];
  const values = node.kind.enum_values ?? labels;
  const selected = values.findIndex((option) =>
    sameJsonValue(option as JsonValue, value)
  );

  const pick = (index: number) => {
    const next = values[index];
    if (next !== undefined) {
      onChange(node.pointer, cloneJsonValue(next as JsonValue));
    }
  };

  if (control === "segmented" || control === "radio") {
    const options = labels.map((label, index) => ({
      value: String(index),
      label,
    }));
    const selectedKey = selected >= 0 ? String(selected) : "";

    if (control === "segmented") {
      return (
        <Segmented
          value={selectedKey}
          onValueChange={(next) => pick(Number(next))}
          options={options}
        />
      );
    }
    return (
      <RadioGroup
        value={selectedKey}
        onValueChange={(next) => pick(Number(next))}
      >
        {options.map((option) => (
          <div key={option.value} className="flex items-center gap-2.5">
            <RadioGroupItem
              value={option.value}
              id={controlId(`${node.pointer}-${option.value}`)}
            />
            <Label
              htmlFor={controlId(`${node.pointer}-${option.value}`)}
              className="text-sm font-normal text-foreground"
            >
              {option.label}
            </Label>
          </div>
        ))}
      </RadioGroup>
    );
  }

  return (
    <Select
      value={selected >= 0 ? String(selected) : ""}
      onValueChange={(next) => pick(Number(next))}
    >
      <SelectTrigger className="w-full">
        <SelectValue placeholder="Select an option" />
      </SelectTrigger>
      <SelectContent>
        {labels.map((label, index) => (
          <SelectItem key={`${label}-${index}`} value={String(index)}>
            {label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/** An empty string is a value for a nullable field and a no-op otherwise. */
function commitText(
  pointer: string,
  next: string,
  nullable: boolean,
  onChange: (pointer: string, value: JsonValue) => void,
) {
  onChange(pointer, next === "" && nullable ? null : next);
}

/** A DOM id derived from the pointer, so a label can point at its control. */
export function controlId(pointer: string): string {
  return `field${pointer.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
}

/**
 * Renders a primitive field control inline (for use in array entries).
 * Returns just the input control without labels or error messages.
 *
 * Enum items never reach this helper: a single value is a select, a set of
 * values is a multi-select.
 */
export function renderSimpleFieldInline(
  fieldKind: InlineFieldKind,
  value: JsonValue | undefined,
  onChange: (value: JsonValue) => void,
): React.ReactNode {
  const resolved = value === undefined ? defaultForKind(fieldKind) : value;
  const nullable = fieldKind.nullable === true;

  switch (fieldKind.scalar) {
    case "integer":
    case "number":
      return (
        <Input
          type="number"
          value={typeof resolved === "number" ? resolved : ""}
          onChange={(event) => {
            if (event.target.value === "" && nullable) {
              onChange(null);
              return;
            }
            onChange(Number(event.target.value));
          }}
          className="h-9"
        />
      );
    case "boolean":
      return (
        <div className="flex items-center gap-2">
          <Switch
            checked={Boolean(resolved)}
            onCheckedChange={(checked) => onChange(checked)}
          />
          <span className="text-xs text-muted-foreground">
            {resolved ? "true" : "false"}
          </span>
        </div>
      );
    case "string":
    default: {
      const text = (resolved as string) ?? "";
      const commit = (next: string) =>
        onChange(next === "" && nullable ? null : next);
      if (fieldKind.multiline) {
        return (
          <Textarea
            rows={3}
            value={text}
            onChange={(event) => commit(event.target.value)}
          />
        );
      }
      return (
        <Input
          type="text"
          value={text}
          onChange={(event) => commit(event.target.value)}
          className="h-9"
        />
      );
    }
  }
}
