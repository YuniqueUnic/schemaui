import { useOverlay } from "./Overlay";
import { variantMatches } from "../utils/variantMatch";
import type { JsonValue, UiNode, UiNodeKind, UiVariant } from "../types";
import { defaultForKind, variantDefault } from "../ui-ast";
import type { ReactNode } from "react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { useState } from "react";

// Helper component for editing array items with local state
function ArrayItemEditor({
  node,
  initialValue,
  errors,
  onSave,
  onClose,
}: {
  node: UiNode;
  initialValue: JsonValue;
  errors: Map<string, string>;
  onSave: (value: JsonValue) => void;
  onClose: () => void;
}) {
  const [localValue, setLocalValue] = useState<JsonValue>(initialValue);

  return (
    <div className="space-y-4">
      <NodeRenderer
        node={node}
        value={localValue}
        errors={errors}
        onChange={(_pointer, newValue) => setLocalValue(newValue)}
        renderMode="inline"
      />
      <div className="flex justify-end gap-2 mt-4">
        <Button onClick={onClose} variant="ghost" size="sm">
          Cancel
        </Button>
        <Button onClick={() => onSave(localValue)} variant="outline" size="sm">
          Done
        </Button>
      </div>
    </div>
  );
}

// Helper component for editing variant entries with local state
function VariantEntryEditor({
  node,
  initialValue,
  errors,
  onSave,
  onClose,
}: {
  node: UiNode;
  initialValue: JsonValue;
  errors: Map<string, string>;
  onSave: (value: JsonValue) => void;
  onClose: () => void;
}) {
  const [localValue, setLocalValue] = useState<JsonValue>(initialValue);

  return (
    <div className="space-y-4">
      <NodeRenderer
        node={node}
        value={localValue}
        errors={errors}
        onChange={(_pointer, newValue) => setLocalValue(newValue)}
        renderMode="inline"
      />
      <div className="flex justify-end gap-2 mt-4">
        <Button onClick={onClose} variant="ghost" size="sm">
          Cancel
        </Button>
        <Button onClick={() => onSave(localValue)} variant="outline" size="sm">
          Done
        </Button>
      </div>
    </div>
  );
}

interface NodeRendererProps {
  node: UiNode;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  renderMode?: "stack" | "inline";
}

export function NodeRenderer(
  { node, value, errors, onChange, renderMode = "stack" }: NodeRendererProps,
) {
  const overlay = useOverlay();
  const error = errors.get(node.pointer);

  const chromeClass = renderMode === "inline"
    ? "space-y-3"
    : node.kind.type === "object"
    ? "space-y-4"
    : "space-y-3 border-b border-theme pb-4";

  return (
    <div className={chromeClass}>
      <header className="flex items-center justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-primary">
            {node.title ?? node.pointer}
            {node.required
              ? (
                <span className="ml-2 text-[10px] uppercase tracking-[0.3em] text-rose-400">
                  required
                </span>
              )
              : null}
          </p>
          {node.description
            ? <p className="text-xs text-muted">{node.description}</p>
            : null}
        </div>
      </header>
      {renderBody(node, value, errors, onChange, overlay)}
      {error ? <p className="text-xs text-rose-400">{error}</p> : null}
    </div>
  );
}

type FieldNode = UiNode & { kind: Extract<UiNodeKind, { type: "field" }> };
type ArrayNode = UiNode & { kind: Extract<UiNodeKind, { type: "array" }> };
type CompositeNode = UiNode & {
  kind: Extract<UiNodeKind, { type: "composite" }>;
};

function renderBody(
  node: UiNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
  overlay: ReturnType<typeof useOverlay>,
): ReactNode {
  switch (node.kind.type) {
    case "field":
      return renderFieldControl(node as FieldNode, value, onChange);
    case "array":
      return renderArrayControl(
        node as ArrayNode,
        value,
        errors,
        onChange,
        overlay,
      );
    case "composite":
      return renderCompositeControl(
        node as CompositeNode,
        value,
        errors,
        onChange,
        overlay,
      );
    case "object":
      return renderObjectControl(node, value, errors, onChange);
    default:
      return null;
  }
}

function renderObjectControl(
  node: UiNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
) {
  if (node.kind.type !== "object") {
    return null;
  }
  return (
    <div className="space-y-4">
      {(node.kind.children ?? []).map((child) => (
        <NodeRenderer
          key={child.pointer}
          node={child}
          value={extractChildValue(value, child.pointer)}
          errors={errors}
          onChange={onChange}
          renderMode="inline"
        />
      ))}
    </div>
  );
}

function renderFieldControl(
  node: FieldNode,
  value: JsonValue | undefined,
  onChange: (pointer: string, value: JsonValue) => void,
) {
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
          onChange={(event) =>
            onChange(node.pointer, Number(event.target.value))}
        />
      );
    case "boolean":
      return (
        <div className="flex items-center gap-3">
          <Switch
            checked={Boolean(resolved)}
            onCheckedChange={(checked) => onChange(node.pointer, checked)}
          />
          <Label className="text-sm text-muted-foreground">Toggle</Label>
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

function renderArrayControl(
  node: ArrayNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
  overlay: ReturnType<typeof useOverlay>,
) {
  const entries = Array.isArray(value) ? (value as JsonValue[]) : [];

  const editEntry = (index: number, initial?: JsonValue) => {
    const entryNode: UiNode = {
      pointer: `${node.pointer}/${index}`,
      title: node.title
        ? `${node.title} entry ${index + 1}`
        : `Entry ${index + 1}`,
      description: node.description,
      required: false,
      default_value: node.default_value,
      kind: node.kind.item,
    };
    overlay.open({
      title: `${node.title ?? node.pointer} · Item ${index + 1}`,
      content: (close) => (
        <ArrayItemEditor
          node={entryNode}
          initialValue={initial ?? entries[index]}
          errors={errors}
          onSave={(newValue) => {
            const next = [...entries];
            next[index] = newValue;
            onChange(node.pointer, next);
            close();
          }}
          onClose={close}
        />
      ),
    });
  };

  const addEntry = () => {
    const placeholder = defaultForKind(node.kind.item);
    const next = [...entries, placeholder];
    onChange(node.pointer, next);
    editEntry(next.length - 1, placeholder);
  };

  const removeEntry = (index: number) => {
    const next = entries.filter((_, idx) => idx !== index);
    onChange(node.pointer, next);
  };

  return (
    <div className="space-y-2">
      {entries.map((entry, index) => (
        <Card
          key={`${node.pointer}-${index}`}
          className="flex items-center justify-between px-3 py-2"
        >
          <span className="truncate text-sm">
            <Badge variant="secondary" className="mr-2">{index + 1}</Badge>
            {formatValueSummary(entry)}
          </span>
          <div className="flex gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => editEntry(index, entry)}
            >
              Edit
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => removeEntry(index)}
              className="text-destructive hover:text-destructive"
            >
              Remove
            </Button>
          </div>
        </Card>
      ))}
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={addEntry}
        className="w-full"
      >
        + Add entry
      </Button>
    </div>
  );
}

function renderCompositeControl(
  node: CompositeNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
  overlay: ReturnType<typeof useOverlay>,
) {
  const { variants, allow_multiple, mode } = node.kind;
  if (!variants.length) {
    return <p className="text-xs text-slate-500">No variants configured.</p>;
  }

  if (allow_multiple) {
    const entries = Array.isArray(value) ? (value as JsonValue[]) : [];
    return (
      <div className="space-y-2">
        {entries.map((entry, index) => {
          const activeVariant = determineVariant(entry, variants);
          return (
            <Card
              key={`${node.pointer}-variant-${index}`}
              className="px-3 py-2"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="text-sm font-medium">
                  {activeVariant?.title ?? `Variant ${index + 1}`}
                </span>
                <div className="flex gap-2">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      const entryNode: UiNode = {
                        pointer: `${node.pointer}/${index}`,
                        title: activeVariant?.title ?? `Variant ${index + 1}`,
                        description: activeVariant?.description,
                        required: false,
                        default_value: node.default_value,
                        kind: activeVariant?.node ?? variants[0].node,
                      };
                      overlay.open({
                        title: `${node.title ?? node.pointer} · Variant entry`,
                        content: (close) => (
                          <VariantEntryEditor
                            node={entryNode}
                            initialValue={entry}
                            errors={errors}
                            onSave={(newValue) => {
                              const next = [...entries];
                              next[index] = newValue;
                              onChange(node.pointer, next);
                              close();
                            }}
                            onClose={close}
                          />
                        ),
                      });
                    }}
                  >
                    Edit
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      const next = entries.filter((_, idx) => idx !== index);
                      onChange(node.pointer, next);
                    }}
                    className="text-destructive hover:text-destructive"
                  >
                    Remove
                  </Button>
                </div>
              </div>
            </Card>
          );
        })}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => {
            const newEntry = variantDefault(variants[0]);
            const next = [...entries, newEntry];
            onChange(node.pointer, next);
            // Open editor for the new entry
            setTimeout(() => {
              const entryNode: UiNode = {
                pointer: `${node.pointer}/${entries.length}`,
                title: variants[0]?.title ?? `Variant ${entries.length + 1}`,
                description: variants[0]?.description,
                required: false,
                default_value: node.default_value,
                kind: variants[0].node,
              };
              overlay.open({
                title: `${node.title ?? node.pointer} · New variant entry`,
                content: (close) => (
                  <VariantEntryEditor
                    node={entryNode}
                    initialValue={newEntry}
                    errors={errors}
                    onSave={(updatedValue) => {
                      const updated = [...next];
                      updated[entries.length] = updatedValue;
                      onChange(node.pointer, updated);
                      close();
                    }}
                    onClose={close}
                  />
                ),
              });
            }, 0);
          }}
          className="w-full"
        >
          + Add variant entry
        </Button>
      </div>
    );
  }

  const activeVariant = determineVariant(value, variants) ?? variants[0];
  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-2 text-xs">
        {variants.map((variant) => (
          <label
            key={variant.id}
            className="inline-flex cursor-pointer items-center gap-2 text-slate-300"
          >
            <input
              type="radio"
              name={node.pointer}
              checked={variant.id === activeVariant.id}
              onChange={() => onChange(node.pointer, variantDefault(variant))}
              className="accent-sky-400"
            />
            <span>{variant.title ?? variant.id}</span>
          </label>
        ))}
      </div>
      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={() =>
          overlay.open({
            title: `${node.title ?? node.pointer} · ${
              activeVariant.title ?? "Variant"
            }`,
            content: (close) => (
              <VariantEntryEditor
                node={{
                  pointer: node.pointer,
                  title: activeVariant.title ?? node.title,
                  description: activeVariant.description ?? node.description,
                  required: node.required,
                  default_value: node.default_value,
                  kind: activeVariant.node,
                }}
                initialValue={value ?? variantDefault(activeVariant)}
                errors={errors}
                onSave={(newValue) => {
                  onChange(node.pointer, newValue);
                  close();
                }}
                onClose={close}
              />
            ),
          })}
        className="w-full mt-2"
      >
        Edit variant ({mode === "one_of" ? "single" : "any"})
      </Button>
    </div>
  );
}

function determineVariant(value: JsonValue | undefined, variants: UiVariant[]) {
  return variants.find((variant) => variantMatches(value, variant.schema)) ??
    variants[0];
}

function extractChildValue(
  container: JsonValue | undefined,
  pointer: string,
): JsonValue | undefined {
  if (container === null || container === undefined) {
    return undefined;
  }
  const token = pointerSegment(pointer);
  if (!token) {
    return undefined;
  }
  if (Array.isArray(container)) {
    const index = Number(token);
    return Number.isNaN(index) ? undefined : container[index];
  }
  if (typeof container === "object") {
    return (container as Record<string, JsonValue>)[token];
  }
  return undefined;
}

function pointerSegment(pointer: string): string | undefined {
  if (!pointer || pointer === "/") {
    return undefined;
  }
  const segments = pointer.split("/").filter(Boolean);
  const raw = segments[segments.length - 1];
  return raw?.replace(/~1/g, "/").replace(/~0/g, "~");
}

function formatValueSummary(value: JsonValue | undefined): string {
  if (value === null || value === undefined) return "empty";
  if (typeof value === "string") return value || '""';
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) return `[items: ${value.length}]`;
  if (typeof value === "object") {
    const keys = Object.keys(value as Record<string, JsonValue>);
    return keys.length ? `{ ${keys.slice(0, 3).join(", ")} }` : "{}";
  }
  return "value";
}
