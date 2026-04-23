import type { JsonValue, UiNode, UiVariant } from "../../types";
import { variantDefault } from "../../ui-ast";
import {
  determineBestVariant,
  determineVariant,
} from "../../utils/variantHelpers";
import { materializeVariantNode } from "../../utils/schemaToUiKind";
import { formatValueSummary, inferValueType } from "../../utils/typeHelpers";
import { setPointerValue } from "../../utils/jsonPointer";
import { VariantSelector } from "../VariantSelector";
import { EntryEditor } from "./shared/EntryEditor";
import { useOverlay } from "../Overlay";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { Card } from "../ui/card";

type CompositeNode = UiNode & {
  kind: Extract<import("../../types").UiNodeKind, { type: "composite" }>;
};

interface CompositeRendererProps {
  node: CompositeNode;
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

/**
 * Renders composite types (oneOf/anyOf) with variant selection
 */
export function CompositeRenderer({
  node,
  value,
  errors,
  onChange,
  renderNode,
}: CompositeRendererProps) {
  const overlay = useOverlay();
  const { variants, allow_multiple } = node.kind;

  if (!variants.length) {
    return (
      <p className="text-xs text-muted-foreground">
        No variants configured.
      </p>
    );
  }

  // Multi-variant mode (allow_multiple = true)
  if (allow_multiple) {
    return (
      <MultiVariantRenderer
        node={node}
        value={value}
        errors={errors}
        onChange={onChange}
        renderNode={renderNode}
        overlay={overlay}
      />
    );
  }

  // Single variant mode
  return (
    <SingleVariantRenderer
      node={node}
      value={value}
      errors={errors}
      onChange={onChange}
      renderNode={renderNode}
    />
  );
}

/**
 * Handles single variant selection (oneOf)
 */
function SingleVariantRenderer({
  node,
  value,
  errors,
  onChange,
  renderNode,
}: Omit<CompositeRendererProps, "overlay">) {
  const { variants, mode } = node.kind;
  const activeVariant = determineBestVariant(value, variants);
  const activeVariantNode = materializeVariantNode(activeVariant);

  const handleVariantChange = (
    changedPointer: string,
    newValue: JsonValue,
  ) => {
    // Only apply nested-path logic when the variant is an object and the
    // current composite value is an object. This preserves arrays / scalars.
    if (
      activeVariant.node.type === "object" &&
      typeof value === "object" && value !== null && !Array.isArray(value)
    ) {
      // Case 1: changed pointer is absolute and inside the composite pointer.
      //   e.g. node.pointer=/a/b and changedPointer=/a/b/settings/timeout
      if (
        node.pointer && changedPointer.startsWith(`${node.pointer}/`)
      ) {
        const relativePath = changedPointer.substring(node.pointer.length);
        const nextValue = setPointerValue(value, relativePath, newValue);
        onChange(node.pointer, nextValue);
        return;
      }

      // Case 2: changed pointer is a relative pointer under the variant
      //   (e.g. /settings/timeout), coming from a variant node built with a
      //   fresh relative pointer space.
      if (
        changedPointer.startsWith("/") &&
        (!node.pointer || !changedPointer.startsWith(node.pointer))
      ) {
        const nextValue = setPointerValue(value, changedPointer, newValue);
        onChange(node.pointer, nextValue);
        return;
      }
    }

    // Fall through: direct replacement (arrays, scalars, full-value updates)
    onChange(changedPointer, newValue);
  };

  return (
    <div className="space-y-4">
      <VariantSelector
        variants={variants}
        mode={mode}
        activeVariantId={activeVariant.id}
        onSelect={(variant) => {
          const newValue = variantDefault(variant);
          onChange(node.pointer, newValue);
        }}
      />

      <div
        key={activeVariant.id}
        className="border-l-2 border-primary/30 pl-4"
      >
        <p className="text-xs text-muted-foreground mb-3">
          {activeVariant.title ?? "Selected Variant"} content:
        </p>
        {renderNode(
          {
            pointer: node.pointer,
            title: undefined,
            description: activeVariant.description,
            required: false,
            default_value: variantDefault(activeVariant),
            kind: activeVariantNode,
          },
          value,
          errors,
          handleVariantChange,
        )}
      </div>
    </div>
  );
}

/**
 * Handles multiple variant entries (anyOf with allow_multiple)
 */
function MultiVariantRenderer({
  node,
  value,
  errors,
  onChange,
  renderNode,
  overlay,
}: CompositeRendererProps & { overlay: ReturnType<typeof useOverlay> }) {
  const { variants } = node.kind;
  const entries = Array.isArray(value) ? (value as JsonValue[]) : [];

  const removeEntry = (index: number) => {
    const next = entries.filter((_, idx) => idx !== index);
    onChange(node.pointer, next);
  };

  const openEntryEditor = (
    index: number,
    entryValue: JsonValue,
    variant: UiVariant,
  ) => {
    const entryNode: UiNode = {
      pointer: `${node.pointer}/${index}`,
      title: variant.title ?? `Variant ${index + 1}`,
      description: variant.description,
      required: false,
      default_value: node.default_value,
      kind: materializeVariantNode(variant),
    };

    overlay.open({
      title: `${node.title ?? node.pointer} · ${variant.title ?? "Entry"} ${
        index + 1
      }`,
      content: (close) => (
        <EntryEditor
          node={entryNode}
          initialValue={entryValue}
          errors={errors}
          onSave={(newValue) => {
            const next = [...entries];
            next[index] = newValue;
            onChange(node.pointer, next);
            close();
          }}
          onClose={close}
          renderNode={renderNode}
        />
      ),
    });
  };

  const addEntry = () => {
    if (variants.length === 1) {
      // Single variant - add directly
      const newEntry = variantDefault(variants[0]);
      const next = [...entries, newEntry];
      onChange(node.pointer, next);
      setTimeout(
        () => openEntryEditor(next.length - 1, newEntry, variants[0]),
        0,
      );
    } else {
      // Multiple variants - show selector
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
                    const newEntry = variantDefault(
                      variant,
                    );
                    const next = [...entries, newEntry];
                    onChange(node.pointer, next);
                    close();
                    setTimeout(() =>
                      openEntryEditor(
                        next.length - 1,
                        newEntry,
                        variant,
                      ), 0);
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
  };

  return (
    <div className="space-y-2">
      {entries.length === 0
        ? (
          <div className="text-center py-6 text-muted-foreground border border-dashed rounded-lg">
            <p className="text-sm">No entries yet</p>
            <p className="text-xs mt-1">
              Click the button below to add your first entry
            </p>
          </div>
        )
        : (
          entries.map((entry, index) => {
            const activeVariant = determineVariant(entry, variants);
            const entryType = inferValueType(entry);
            return (
              <Card
                key={`${node.pointer}-variant-${index}`}
                className="px-3 py-2"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 flex-1 space-y-1">
                    <div className="flex items-center gap-2 flex-1 min-w-0">
                      <Badge
                        variant="secondary"
                        className="shrink-0"
                      >
                        {index + 1}
                      </Badge>
                      <Badge
                        variant="outline"
                        className="font-mono text-xs shrink-0"
                      >
                        {entryType}
                      </Badge>
                      <span className="text-sm font-medium truncate">
                        {activeVariant?.title ??
                          `Variant ${index + 1}`}
                      </span>
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
                      onClick={() =>
                        openEntryEditor(
                          index,
                          entry,
                          activeVariant ??
                            variants[0],
                        )}
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
        + Add variant entry
      </Button>
    </div>
  );
}
