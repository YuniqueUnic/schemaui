import type { JsonValue, UiNode } from "../../types";
import { defaultForKind } from "../../ui-ast";
import {
  formatValueSummary,
  inferValueType,
  isEnumFieldKind,
  isSimpleKind,
  type InlineFieldKind,
} from "../../utils/typeHelpers";
import {
  determineVariant,
} from "../../utils/variantHelpers";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { Card } from "../ui/card";
import { useOverlay } from "../Overlay";
import { materializeCompositeKind } from "../../utils/schemaToUiKind";
import { renderSimpleFieldInline } from "./FieldRenderer";
import { MultiSelectRenderer } from "./MultiSelectRenderer";
import { EntryEditor } from "./shared/EntryEditor";

/**
 * Array Renderer - Handles all array rendering logic
 * Supports both inline editing (simple types) and dialog editing (complex types)
 */

type ArrayNode = UiNode & {
  kind: Extract<import("../../types").UiNodeKind, { type: "array" }>;
};

interface ArrayRendererProps {
  node: ArrayNode;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  renderNode: (
    node: UiNode,
    value: JsonValue | undefined,
    errors: Map<string, string>,
    onChange: (pointer: string, value: JsonValue) => void,
  ) => React.ReactNode;
}

export function ArrayRenderer({
  node,
  value,
  errors,
  onChange,
  renderNode,
}: ArrayRendererProps) {
  const overlay = useOverlay();
  const entries = Array.isArray(value) ? (value as JsonValue[]) : [];
  const itemKind = node.kind.item;

  const removeEntry = (index: number) => {
    const next = entries.filter((_, idx) => idx !== index);
    onChange(node.pointer, next);
  };

  // Array of enum options: a set of choices, so a single multi-select control.
  if (isEnumFieldKind(itemKind)) {
    return (
      <MultiSelectRenderer
        node={node}
        itemKind={itemKind}
        value={value}
        onChange={onChange}
      />
    );
  }

  // Array of primitives: edit each entry inline.
  if (isSimpleKind(itemKind)) {
    return (
      <SimpleArrayRenderer
        node={node}
        entries={entries}
        itemKind={itemKind}
        onChange={onChange}
        removeEntry={removeEntry}
      />
    );
  }

  // Array of structured entries: one overlay per entry.
  return (
    <ComplexArrayRenderer
      node={node}
      entries={entries}
      itemKind={itemKind}
      errors={errors}
      onChange={onChange}
      removeEntry={removeEntry}
      overlay={overlay}
      renderNode={renderNode}
    />
  );
}

/**
 * Simple Array Renderer - Inline editing for primitive types
 */
interface SimpleArrayRendererProps {
  node: ArrayNode;
  entries: JsonValue[];
  itemKind: InlineFieldKind;
  onChange: (pointer: string, value: JsonValue) => void;
  removeEntry: (index: number) => void;
}

function SimpleArrayRenderer({
  node,
  entries,
  itemKind,
  onChange,
  removeEntry,
}: SimpleArrayRendererProps) {
  const addEntry = () => {
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
      {entries.length === 0
        ? (
          <div className="text-center py-4 text-muted-foreground border border-dashed rounded-lg">
            <p className="text-sm">No items yet</p>
            <p className="text-xs mt-1">
              Click below to add your first item
            </p>
          </div>
        )
        : (
          entries.map((entry, index) => (
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
          ))
        )}
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
 * Complex Array Renderer - Dialog editing for complex types
 */
interface ComplexArrayRendererProps {
  node: ArrayNode;
  entries: JsonValue[];
  itemKind: import("../../types").UiNodeKind;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  removeEntry: (index: number) => void;
  overlay: ReturnType<typeof useOverlay>;
  renderNode: (
    node: UiNode,
    value: JsonValue | undefined,
    errors: Map<string, string>,
    onChange: (pointer: string, value: JsonValue) => void,
  ) => React.ReactNode;
}

function ComplexArrayRenderer({
  node,
  entries,
  itemKind,
  errors,
  onChange,
  removeEntry,
  overlay,
  renderNode,
}: ComplexArrayRendererProps) {
  const editorKind = materializeCompositeKind(itemKind);

  const openEntryEditor = (
    index: number,
    initialValue: JsonValue,
    onSaveEntry: (value: JsonValue) => void,
  ) => {
    const entryNode: UiNode = {
      pointer: `${node.pointer}/${index}`,
      title: node.title
        ? `${node.title} entry ${index + 1}`
        : `Entry ${index + 1}`,
      description: node.description,
      required: false,
      default_value: node.default_value,
      kind: editorKind,
    };

    overlay.open({
      title: `${node.title ?? node.pointer} · Item ${index + 1}`,
      content: (close) => (
        <EntryEditor
          node={entryNode}
          initialValue={initialValue}
          errors={errors}
          onSave={(newValue: JsonValue) => {
            onSaveEntry(newValue);
            close();
          }}
          onClose={close}
          renderNode={renderNode}
        />
      ),
    });
  };

  const editEntry = (index: number) => {
    openEntryEditor(index, entries[index], (newValue) => {
      const next = [...entries];
      next[index] = newValue;
      onChange(node.pointer, next);
    });
  };

  const addEntry = () => {
    const draftValue = defaultForKind(itemKind);
    const draftIndex = entries.length;
    openEntryEditor(draftIndex, draftValue, (newValue) => {
      onChange(node.pointer, [...entries, newValue]);
    });
  };

  return (
    <div className="space-y-2">
      {entries.length === 0
        ? (
          <div className="text-center py-6 text-muted-foreground border border-dashed rounded-lg">
            <p className="text-sm">No items yet</p>
            <p className="text-xs mt-1">
              Click below to add your first item
            </p>
          </div>
        )
        : (
          entries.map((entry, index) => {
            const activeVariant = itemKind.type === "composite"
              ? determineVariant(entry, itemKind.variants)
              : undefined;
            const entryType = activeVariant?.title ?? inferValueType(entry);
            return (
              <Card
                key={`${node.pointer}-${index}`}
                className="flex items-start justify-between gap-3 px-3 py-2"
              >
                <div className="min-w-0 flex-1 space-y-1">
                  <div className="flex items-center gap-2 min-w-0">
                    <Badge variant="secondary">
                      {index + 1}
                    </Badge>
                    <Badge
                      variant="outline"
                      className="font-mono text-xs"
                    >
                      {entryType}
                    </Badge>
                  </div>
                  <p className="text-xs text-muted-foreground break-words">
                    {formatValueSummary(entry)}
                  </p>
                </div>
                <div className="flex gap-1 shrink-0">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => editEntry(index)}
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
          })
        )}
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
