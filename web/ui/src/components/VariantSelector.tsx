import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { CompositeMode, UiVariant } from "../types";
import { CheckCircle2, Circle } from "lucide-react";

interface VariantSelectorProps {
  variants: UiVariant[];
  mode: CompositeMode;
  activeVariantId: string | undefined;
  onSelect: (variant: UiVariant) => void;
  onEdit?: () => void;
}

export function VariantSelector({
  variants,
  mode,
  activeVariantId,
  onSelect,
  onEdit,
}: VariantSelectorProps) {
  if (!variants.length) {
    return (
      <p className="text-xs text-muted-foreground">
        No variants configured.
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {/* Variant Cards */}
      <div className="space-y-2">
        {variants.map((variant) => {
          const isActive = variant.id === activeVariantId;

          return (
            <Card
              key={variant.id}
              className={`
                p-2 cursor-pointer transition-all hover:border-primary/50
                ${
                isActive
                  ? "border-primary bg-primary/5 shadow-sm"
                  : "border-border hover:bg-accent/50"
              }
              `}
              onClick={() => onSelect(variant)}
            >
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  {/* Selection Indicator */}
                  <div className="shrink-0">
                    {isActive
                      ? <CheckCircle2 className="h-4 w-4 text-primary" />
                      : <Circle className="h-4 w-4 text-muted-foreground" />}
                  </div>

                  {/* Variant Info */}
                  <div className="flex-1 min-w-0 flex items-center gap-2">
                    <span className="text-xs font-mono text-muted-foreground shrink-0">
                      {formatTypeInfo(variant)}
                    </span>
                    <span className="text-sm truncate">
                      {variant.title || variant.id}
                    </span>
                  </div>
                </div>
                {/* Description */}
                {variant.description && (
                  <div className="pl-6 text-[11px] text-muted-foreground line-clamp-1">
                    {variant.description}
                  </div>
                )}
              </div>
            </Card>
          );
        })}
      </div>

      {/* Edit Button (optional, only shown when onEdit is provided) */}
      {onEdit && activeVariantId && (
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onEdit}
          className="w-full"
        >
          Edit {mode === "one_of" ? "Selected" : "Active"} Variant
        </Button>
      )}

      {/* Help Text */}
      <p className="text-[11px] text-muted-foreground">
        {mode === "one_of"
          ? "Select a variant type. The corresponding editor will appear below."
          : "This field can match multiple schemas"}
      </p>
    </div>
  );
}

function formatTypeInfo(variant: UiVariant): string {
  const node = variant.node;

  switch (node.type) {
    case "field": {
      if (node.enum_options?.length) {
        return `enum(${node.enum_options.length})`;
      }
      return node.scalar;
    }

    case "array": {
      return `${formatNodeType(node.item)}[]`;
    }

    case "object": {
      const children = node.children || [];
      if (children.length === 0) {
        return "object";
      }
      // Show property names for better clarity
      const propNames = children.slice(0, 2).map((c) => {
        const segments = c.pointer.split("/").filter(Boolean);
        return segments[segments.length - 1] || "";
      }).filter(Boolean);

      if (children.length > 2) {
        return `{${propNames.join(", ")}...}`;
      }
      return `{${propNames.join(", ")}}`;
    }

    case "composite": {
      return node.mode === "one_of" ? "oneOf" : "anyOf";
    }

    default:
      return "unknown";
  }
}

function formatNodeType(
  nodeKind: import("../types").UiNodeKind | undefined,
): string {
  if (!nodeKind) return "any";

  switch (nodeKind.type) {
    case "field":
      return nodeKind.scalar || "field";
    case "array":
      return "array";
    case "object":
      return "object";
    case "composite":
      return nodeKind.mode === "one_of" ? "oneOf" : "anyOf";
    default:
      return "unknown";
  }
}
