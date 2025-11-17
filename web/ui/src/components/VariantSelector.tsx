import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
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

    const modeLabel = mode === "one_of"
        ? "OneOf (select exactly one)"
        : "AnyOf (value satisfies at least one)";

    const modeColor = mode === "one_of"
        ? "bg-purple-500/10 text-purple-400"
        : "bg-blue-500/10 text-blue-400";

    return (
        <div className="space-y-3">
            {/* Mode Label */}
            <div className="flex items-center gap-2">
                <Badge variant="outline" className={modeColor}>
                    {modeLabel}
                </Badge>
            </div>

            {/* Variant Cards */}
            <div className="space-y-2">
                {variants.map((variant, index) => {
                    const isActive = variant.id === activeVariantId;

                    return (
                        <Card
                            key={variant.id}
                            className={`
                p-4 cursor-pointer transition-all hover:border-primary/50
                ${
                                isActive
                                    ? "border-primary bg-primary/5 shadow-md"
                                    : "border-border hover:bg-accent/50"
                            }
              `}
                            onClick={() => onSelect(variant)}
                        >
                            <div className="flex items-start gap-3">
                                {/* Selection Indicator */}
                                <div className="pt-0.5">
                                    {isActive
                                        ? (
                                            <CheckCircle2 className="h-5 w-5 text-primary" />
                                        )
                                        : (
                                            <Circle className="h-5 w-5 text-muted-foreground" />
                                        )}
                                </div>

                                {/* Variant Info */}
                                <div className="flex-1 min-w-0">
                                    <div className="flex items-center gap-2 mb-1">
                                        <span className="text-sm font-semibold text-foreground">
                                            #{index + 1}{" "}
                                            {variant.title || variant.id}
                                        </span>
                                        {isActive && (
                                            <Badge
                                                variant="secondary"
                                                className="text-[10px] px-1.5 py-0"
                                            >
                                                Active
                                            </Badge>
                                        )}
                                    </div>

                                    {/* Type Information */}
                                    <div className="flex items-center gap-2 mb-2">
                                        <Badge
                                            variant="outline"
                                            className="text-[10px] font-mono"
                                        >
                                            {formatTypeInfo(variant)}
                                        </Badge>
                                        {variant.is_object && (
                                            <Badge
                                                variant="outline"
                                                className="text-[10px]"
                                            >
                                                object
                                            </Badge>
                                        )}
                                    </div>

                                    {/* Description */}
                                    {variant.description && (
                                        <p className="text-xs text-muted-foreground line-clamp-2">
                                            {variant.description}
                                        </p>
                                    )}
                                </div>
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
        case "field":
            if (node.enum_options?.length) {
                return `enum(${node.enum_options.length})`;
            }
            return node.scalar;

        case "array":
            return `${formatNodeType(node.item)}[]`;

        case "object":
            const childCount = node.children?.length || 0;
            return childCount > 0 ? `object(${childCount})` : "object";

        case "composite":
            return node.mode === "one_of" ? "oneOf" : "anyOf";

        default:
            return "unknown";
    }
}

function formatNodeType(nodeKind: any): string {
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
            return nodeKind.type || "unknown";
    }
}
