import { useOverlay } from "./Overlay";
import type { JsonValue, UiNode, UiNodeKind } from "../types";
import { defaultForKind, variantDefault } from "../ui-ast";
import type { ReactNode } from "react";
import { VariantSelector } from "./VariantSelector";
import {
  determineBestVariant,
  determineVariant,
} from "../utils/variantHelpers";
import {
  extractChildValue,
  formatValueSummary,
  inferValueType,
  isSimpleKind,
} from "../utils/typeHelpers";

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
import {
  joinPointer,
  setPointerValue,
  splitPointer,
} from "../utils/jsonPointer";

// ============================================================================
// Helper Components
// ============================================================================

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
        onChange={(changedPointer, newValue) =>
          setLocalValue((previous) =>
            applyLocalChangeForEditor(
              node.pointer,
              previous,
              changedPointer,
              newValue,
            )
          )}
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
        onChange={(changedPointer, newValue) =>
          setLocalValue((previous) =>
            applyLocalChangeForEditor(
              node.pointer,
              previous,
              changedPointer,
              newValue,
            )
          )}
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
  const itemKind = node.kind.item;

  // Check if array items are simple types that can be rendered inline
  const isSimpleItemType = isSimpleKind(itemKind);

  const removeEntry = (index: number) => {
    const next = entries.filter((_, idx) => idx !== index);
    onChange(node.pointer, next);
  };

  // -------------------------
  // Simple Type Array Rendering (Inline)
  // -------------------------
  if (isSimpleItemType && itemKind.type === "field") {
    const addSimpleEntry = () => {
      const placeholder = defaultForKind(itemKind);
      const next = [...entries, placeholder];
      onChange(node.pointer, next);
    };

    const updateEntry = (index: number, newValue: JsonValue) => {
      const next = [...entries];
      next[index] = newValue;
      onChange(node.pointer, next);
    };

    return (
      <div className="space-y-2">
        {entries.map((entry, index) => (
          <div
            key={`${node.pointer}-${index}`}
            className="flex items-center gap-2"
          >
            <Badge variant="secondary" className="shrink-0">
              {index + 1}
            </Badge>
            <div className="flex-1">
              {renderSimpleFieldInline(
                itemKind,
                entry,
                (newValue) => updateEntry(index, newValue),
              )}
            </div>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => removeEntry(index)}
              className="text-destructive hover:text-destructive shrink-0"
            >
              Remove
            </Button>
          </div>
        ))}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={addSimpleEntry}
          className="w-full"
        >
          + Add entry
        </Button>
      </div>
    );
  }

  // -------------------------
  // Complex Type Array Rendering (With Dialog)
  // -------------------------
  const editEntry = (index: number, initial?: JsonValue) => {
    const entryNode: UiNode = {
      pointer: `${node.pointer}/${index}`,
      title: node.title
        ? `${node.title} entry ${index + 1}`
        : `Entry ${index + 1}`,
      description: node.description,
      required: false,
      default_value: node.default_value,
      kind: itemKind,
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
    const placeholder = defaultForKind(itemKind);
    const next = [...entries, placeholder];
    onChange(node.pointer, next);
    editEntry(next.length - 1, placeholder);
  };

  return (
    <div className="space-y-2">
      {entries.map((entry, index) => {
        const entryType = inferValueType(entry);
        return (
          <Card
            key={`${node.pointer}-${index}`}
            className="flex items-center justify-between px-3 py-2"
          >
            <div className="flex items-center gap-2 truncate flex-1">
              <Badge variant="secondary">{index + 1}</Badge>
              <Badge variant="outline" className="font-mono text-xs">
                {entryType}
              </Badge>
              <span className="truncate text-sm">
                {formatValueSummary(entry)}
              </span>
            </div>
            <div className="flex gap-2 shrink-0">
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
        );
      })}
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

/**
 * Renders a simple field control inline (for use in arrays)
 * Returns just the input control without labels or error messages
 */
function renderSimpleFieldInline(
  fieldKind: Extract<UiNodeKind, { type: "field" }>,
  value: JsonValue | undefined,
  onChange: (value: JsonValue) => void,
): ReactNode {
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
          const entryType = inferValueType(entry);
          return (
            <Card
              key={`${node.pointer}-variant-${index}`}
              className="px-3 py-2"
            >
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 flex-1">
                  <Badge variant="secondary">{index + 1}</Badge>
                  <Badge variant="outline" className="font-mono text-xs">
                    {entryType}
                  </Badge>
                  <span className="text-sm font-medium truncate">
                    {activeVariant?.title ?? `Variant ${index + 1}`}
                  </span>
                </div>
                <div className="flex gap-2 shrink-0">
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
            // If there's only one variant, use it directly
            if (variants.length === 1) {
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
            } else {
              // Multiple variants - show a selector first
              overlay.open({
                title: `Select variant type for ${node.title ?? node.pointer}`,
                content: (close) => (
                  <div className="space-y-4">
                    <p className="text-sm text-muted-foreground">
                      Choose which type of item to add:
                    </p>
                    <div className="space-y-2">
                      {variants.map((variant) => (
                        <Button
                          key={variant.id}
                          type="button"
                          variant="outline"
                          className="w-full justify-start"
                          onClick={() => {
                            const newEntry = variantDefault(variant);
                            const next = [...entries, newEntry];
                            onChange(node.pointer, next);
                            close();

                            // Open editor for the new entry
                            setTimeout(() => {
                              const entryNode: UiNode = {
                                pointer: `${node.pointer}/${entries.length}`,
                                title: variant.title ??
                                  `Variant ${entries.length + 1}`,
                                description: variant.description,
                                required: false,
                                default_value: node.default_value,
                                kind: variant.node,
                              };
                              overlay.open({
                                title: `${node.title ?? node.pointer} · New ${
                                  variant.title ?? "variant"
                                } entry`,
                                content: (closeEditor) => (
                                  <VariantEntryEditor
                                    node={entryNode}
                                    initialValue={newEntry}
                                    errors={errors}
                                    onSave={(updatedValue) => {
                                      const updated = [...next];
                                      updated[entries.length] = updatedValue;
                                      onChange(node.pointer, updated);
                                      closeEditor();
                                    }}
                                    onClose={closeEditor}
                                  />
                                ),
                              });
                            }, 0);
                          }}
                        >
                          <span className="font-medium">
                            {variant.title ?? variant.id}
                          </span>
                          {variant.description && (
                            <span className="ml-2 text-xs text-muted-foreground">
                              {variant.description}
                            </span>
                          )}
                        </Button>
                      ))}
                    </div>
                  </div>
                ),
              });
            }
          }}
          className="w-full"
        >
          + Add variant entry
        </Button>
      </div>
    );
  }

  const activeVariant = determineBestVariant(value, variants);

  // Debug logging for variant matching
  if (
    typeof window !== "undefined" && window.location.hostname === "localhost"
  ) {
    console.log("renderCompositeControl:", {
      value,
      activeVariantId: activeVariant.id,
      activeVariantTitle: activeVariant.title,
    });
  }

  return (
    <div className="space-y-4">
      {/* Variant Selector (Radio Group) */}
      <VariantSelector
        variants={variants}
        mode={mode}
        activeVariantId={activeVariant.id}
        onSelect={(variant) => {
          // Generate a unique default value for the selected variant
          const newValue = variantDefault(variant);
          onChange(node.pointer, newValue);
        }}
      />

      {/* Direct rendering of the selected variant's content */}
      <div key={activeVariant.id} className="border-l-2 border-primary/30 pl-4">
        <p className="text-xs text-muted-foreground mb-3">
          {activeVariant.title ?? "Selected Variant"} content:
        </p>
        <NodeRenderer
          node={{
            pointer: node.pointer,
            title: undefined, // Don't show title again as it's shown above
            description: activeVariant.description,
            required: false,
            default_value: node.default_value,
            kind: activeVariant.node,
          }}
          value={value}
          errors={errors}
          onChange={(changedPointer, newValue) => {
            // Debug logging
            console.log("Composite onChange:", {
              changedPointer,
              nodePointer: node.pointer,
              newValue,
              currentValue: value,
              variantType: activeVariant.node.type,
            });

            // For composite types with object values, we need to handle nested updates
            if (
              activeVariant.node.type === "object" &&
              typeof value === "object" && value !== null
            ) {
              // Check if the changed pointer is a child of the node pointer
              // Both absolute and relative paths should be handled

              // Case 1: Absolute path (e.g., /e/e1/e2/e3/e4/logic/value)
              if (changedPointer.startsWith(node.pointer + "/")) {
                const fieldPath = changedPointer.substring(node.pointer.length);
                const fieldName = fieldPath.substring(1).split("/")[0];
                console.log(
                  "Absolute path - Updating field:",
                  fieldName,
                  "with value:",
                  newValue,
                );

                // Update only the specific field in the object
                const updatedValue = { ...value, [fieldName]: newValue };
                console.log("Updated object:", updatedValue);

                onChange(node.pointer, updatedValue);
                return;
              }

              // Case 2: Relative path (e.g., /value)
              // Only handle if it's a shallow relative path (no nested slashes)
              if (
                changedPointer.startsWith("/") &&
                !changedPointer.startsWith(node.pointer) &&
                changedPointer.substring(1).indexOf("/") === -1 // No nested slashes
              ) {
                const fieldName = changedPointer.substring(1);
                console.log(
                  "Relative path - Updating field:",
                  fieldName,
                  "with value:",
                  newValue,
                );

                // Update only the specific field in the object
                const updatedValue = { ...value, [fieldName]: newValue };
                console.log("Updated object:", updatedValue);

                onChange(node.pointer, updatedValue);
                return;
              }

              // Case 3: Direct replacement
              console.log("Direct replacement of entire value");
              onChange(changedPointer, newValue);
            } else {
              // For non-object types, pass through as-is
              console.log("Non-object type, passing through");
              onChange(changedPointer, newValue);
            }
          }}
          renderMode="inline"
        />
      </div>
    </div>
  );
}

function applyLocalChangeForEditor(
  rootPointer: string,
  currentValue: JsonValue,
  changedPointer: string,
  newValue: JsonValue,
): JsonValue {
  if (
    !changedPointer || changedPointer === rootPointer || changedPointer === "/"
  ) {
    return newValue;
  }

  const rootSegments = splitPointer(rootPointer);
  const changedSegments = splitPointer(changedPointer);

  const hasCommonPrefix = rootSegments.length > 0 &&
    rootSegments.length <= changedSegments.length &&
    rootSegments.every((segment, index) => segment === changedSegments[index]);

  const relativeSegments = hasCommonPrefix
    ? changedSegments.slice(rootSegments.length)
    : changedSegments;

  const relativePointer = joinPointer(relativeSegments);
  return setPointerValue(currentValue, relativePointer, newValue);
}
