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
import { RichSurface } from "../rich/RichSurface";
import { MermaidEditorControl } from "../rich/MermaidEditorControl";
import type { InlineFieldKind } from "../../utils/typeHelpers";
import { resolveBounds, resolveControl, snapToBounds } from "../../lib/control";
import { useI18n } from "../../i18n";

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
  const { t } = useI18n();
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

    case "mermaid": {
      const text = typeof resolved === "string" ? resolved : "";
      return (
        <MermaidEditorControl
          value={text}
          onChange={(next) => commitText(node.pointer, next, nullable, onChange)}
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
            {on ? t("On") : t("Off")}
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
            {checked ? t("Enabled") : t("Disabled")}
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
  const baseLabels = node.kind.enum_options ?? [];
  const values = node.kind.enum_values ?? baseLabels;
  const details = node.kind.enum_details;
  const { t } = useI18n();
  // An `x-options` label overrides the derived one, by index.
  const labelFor = (index: number) => details?.[index]?.label ?? baseLabels[index] ?? "";
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
    const options = baseLabels.map((_, index) => ({
      value: String(index),
      label: labelFor(index),
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
        {options.map((option, index) => {
          const detail = details?.[index];
          return (
            <div key={option.value} className="flex items-start gap-2.5">
              <RadioGroupItem
                value={option.value}
                id={controlId(`${node.pointer}-${option.value}`)}
                className="mt-0.5"
              />
              <div className="min-w-0 flex-1 space-y-1.5">
                <Label
                  htmlFor={controlId(`${node.pointer}-${option.value}`)}
                  className="block text-sm font-normal text-foreground"
                >
                  {option.label}
                </Label>
                {detail?.description && (
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    {detail.description}
                  </p>
                )}
                {detail?.content && (
                  <RichSurface
                    content={detail.content}
                    variant="thumb"
                    title={option.label}
                  />
                )}
              </div>
            </div>
          );
        })}
      </RadioGroup>
    );
  }

  return (
    <div className="space-y-2">
      <Select
        value={selected >= 0 ? String(selected) : ""}
        onValueChange={(next) => pick(Number(next))}
      >
        <SelectTrigger className="w-full">
          {/* Text-only on purpose: Radix mirrors the selected item's children
              into the trigger, and a figure taller than the trigger row is
              what overflowed it. The selected option's figure lives below
              instead, where it has room and the full interactions. */}
          <SelectValue placeholder={t("Select an option")}>
            {selected >= 0
              ? labelFor(selected)
              : <span className="text-muted-foreground">{t("Select an option")}</span>}
          </SelectValue>
        </SelectTrigger>
        <SelectContent>
          {baseLabels.map((_, index) => {
            const detail = details?.[index];
            return (
              <SelectItem key={`${index}-${labelFor(index)}`} value={String(index)}>
                <span className="flex items-center gap-2">
                  {detail?.content && (
                    <RichSurface
                      content={detail.content}
                      variant="mini"
                      title={labelFor(index)}
                    />
                  )}
                  {labelFor(index)}
                </span>
              </SelectItem>
            );
          })}
        </SelectContent>
      </Select>
      {selected >= 0 && details?.[selected]?.content && (
        <RichSurface
          content={details[selected].content}
          variant="thumb"
          title={labelFor(selected)}
        />
      )}
    </div>
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
