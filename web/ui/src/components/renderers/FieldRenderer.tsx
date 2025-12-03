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

type FieldNode = UiNode & {
  kind: Extract<import("../../types").UiNodeKind, { type: "field" }>;
};

interface FieldRendererProps {
  node: FieldNode;
  value: JsonValue | undefined;
  onChange: (pointer: string, value: JsonValue) => void;
}

/**
 * Renders field controls (string, number, boolean, enum)
 */
export function FieldRenderer({ node, value, onChange }: FieldRendererProps) {
  const resolved = value ?? node.default_value ?? defaultForKind(node.kind);

  if (node.kind.enum_options?.length) {
    return (
      <Select
        value={(resolved as string) ?? ""}
        onValueChange={(newValue) => onChange(node.pointer, newValue)}
      >
        <SelectTrigger className="w-full">
          <SelectValue placeholder="Select an option" />
        </SelectTrigger>
        <SelectContent>
          {node.kind.enum_options.map((option) => (
            <SelectItem key={option} value={option}>
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
          value={typeof resolved === "number" ? resolved : 0}
          onChange={(event) => {
            const numValue = event.target.value === ""
              ? 0
              : Number(event.target.value);
            if (!isNaN(numValue)) {
              onChange(node.pointer, numValue);
            }
          }}
          onBlur={(event) => {
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
    default:
      return (
        <Input
          type="text"
          value={(resolved as string) ?? ""}
          onChange={(event) => onChange(node.pointer, event.target.value)}
        />
      );
  }
}

/**
 * Renders a simple field control inline (for use in arrays)
 * Returns just the input control without labels or error messages
 */
export function renderSimpleFieldInline(
  fieldKind: Extract<import("../../types").UiNodeKind, { type: "field" }>,
  value: JsonValue | undefined,
  onChange: (value: JsonValue) => void,
): React.ReactNode {
  const resolved = value ?? defaultForKind(fieldKind);

  switch (fieldKind.scalar) {
    case "integer":
    case "number":
      return (
        <Input
          type="number"
          value={typeof resolved === "number" ? resolved : 0}
          onChange={(event) => onChange(Number(event.target.value))}
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
    default:
      return (
        <Input
          type="text"
          value={(resolved as string) ?? ""}
          onChange={(event) => onChange(event.target.value)}
          className="h-9"
        />
      );
  }
}
