import { useState } from "react";
import type { JsonValue, UiNode } from "../../types";
import { defaultForKind } from "../../ui-ast";
import {
    formatValueSummary,
    inferValueType,
    isSimpleKind,
} from "../../utils/typeHelpers";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { Card } from "../ui/card";
import { Input } from "../ui/input";
import { Switch } from "../ui/switch";
import { useOverlay } from "../Overlay";
import {
    joinPointer,
    setPointerValue,
    splitPointer,
} from "../../utils/jsonPointer";

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
    const isSimpleItemType = isSimpleKind(itemKind);

    const removeEntry = (index: number) => {
        const next = entries.filter((_, idx) => idx !== index);
        onChange(node.pointer, next);
    };

    // Simple Type Array (Inline Editing)
    if (isSimpleItemType && itemKind.type === "field") {
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

    // Complex Type Array (Dialog Editing)
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
    itemKind: Extract<import("../../types").UiNodeKind, { type: "field" }>;
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
                    onSave={(newValue: JsonValue) => {
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
                            <Badge
                                variant="outline"
                                className="font-mono text-xs"
                            >
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
 * Array Item Editor - Local state editor for array items
 * Note: This will be connected to NodeRenderer via props to avoid circular dependency
 */
interface ArrayItemEditorProps {
    node: UiNode;
    initialValue: JsonValue;
    errors: Map<string, string>;
    onSave: (value: JsonValue) => void;
    onClose: () => void;
    renderNode: (
        node: UiNode,
        value: JsonValue,
        errors: Map<string, string>,
        onChange: (pointer: string, value: JsonValue) => void,
    ) => React.ReactNode;
}

function ArrayItemEditor({
    node,
    initialValue,
    errors,
    onSave,
    onClose,
    renderNode,
}: ArrayItemEditorProps) {
    const [localValue, setLocalValue] = useState<JsonValue>(initialValue);

    return (
        <div className="space-y-4">
            {renderNode(
                node,
                localValue,
                errors,
                (changedPointer: string, newValue: JsonValue) =>
                    setLocalValue((previous) =>
                        applyLocalChange(
                            node.pointer,
                            previous,
                            changedPointer,
                            newValue,
                        )
                    ),
            )}
            <div className="flex justify-end gap-2 mt-4">
                <Button onClick={onClose} variant="ghost" size="sm">
                    Cancel
                </Button>
                <Button
                    onClick={() => onSave(localValue)}
                    variant="outline"
                    size="sm"
                >
                    Done
                </Button>
            </div>
        </div>
    );
}

/**
 * Helper: Apply local change within editor scope
 */
function applyLocalChange(
    rootPointer: string,
    currentValue: JsonValue,
    changedPointer: string,
    newValue: JsonValue,
): JsonValue {
    if (
        !changedPointer || changedPointer === rootPointer ||
        changedPointer === "/"
    ) {
        return newValue;
    }

    const rootSegments = splitPointer(rootPointer);
    const changedSegments = splitPointer(changedPointer);

    const hasCommonPrefix = rootSegments.length > 0 &&
        rootSegments.length <= changedSegments.length &&
        rootSegments.every((segment, index) =>
            segment === changedSegments[index]
        );

    const relativeSegments = hasCommonPrefix
        ? changedSegments.slice(rootSegments.length)
        : changedSegments;

    const relativePointer = joinPointer(relativeSegments);
    return setPointerValue(currentValue, relativePointer, newValue);
}

/**
 * Helper: Render simple field inline
 */
function renderSimpleFieldInline(
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
