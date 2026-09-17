/* eslint-disable react-refresh/only-export-components */

import type { JsonValue, UiNode } from "../../types";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Switch } from "../ui/switch";
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
 * Renders field controls (string, number, boolean, enum)
 */
export function FieldRenderer({ node, value, onChange }: FieldRendererProps) {
  const resolved = value === undefined
    ? (node.default_value ?? defaultForKind(node.kind))
    : value;
  const nullable = node.kind.nullable === true;

  if (node.kind.enum_options?.length) {
    const enumValues = node.kind.enum_values ?? node.kind.enum_options;
    const selectedIndex = enumValues.findIndex((option) =>
      sameJsonValue(option as JsonValue, resolved)
    );
    return (
      <Select
        value={selectedIndex >= 0 ? String(selectedIndex) : ""}
        onValueChange={(newValue) => {
          const next = enumValues[Number(newValue)];
          if (next !== undefined) {
            onChange(node.pointer, cloneJsonValue(next as JsonValue));
          }
        }}
      >
        <SelectTrigger className="w-full">
          <SelectValue placeholder="Select an option" />
        </SelectTrigger>
        <SelectContent>
          {node.kind.enum_options.map((option, index) => (
            <SelectItem key={`${option}-${index}`} value={String(index)}>
              {option}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    );
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
      return (
        <div className="flex items-center gap-3">
          <Switch
            checked={Boolean(resolved)}
            onCheckedChange={(checked) => onChange(node.pointer, checked)}
          />
          <Label className="text-sm text-muted-foreground">
            Toggle
          </Label>
        </div>
      );
    case "string":
    default: {
      const text = (resolved as string) ?? "";
      const commit = (next: string) =>
        onChange(node.pointer, next === "" && nullable ? null : next);
      if (node.kind.multiline) {
        return (
          <Textarea
            rows={4}
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
        />
      );
    }
  }
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
