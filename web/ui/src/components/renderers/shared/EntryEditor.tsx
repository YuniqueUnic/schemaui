import { useState } from "react";
import type { JsonValue, UiNode } from "../../../types";
import { Button } from "../../ui/button";
import {
    joinPointer,
    setPointerValue,
    splitPointer,
} from "../../../utils/jsonPointer";

/**
 * Generic entry editor for array items and variant entries
 * Replaces both ArrayItemEditor and VariantEntryEditor
 */
interface EntryEditorProps {
    node: UiNode;
    initialValue: JsonValue;
    errors: Map<string, string>;
    onSave: (value: JsonValue) => void;
    onClose: () => void;
    renderNode: (
        node: UiNode,
        value: JsonValue,
        errors: Map<string, string>,
        onChange: (pointer: string, newValue: JsonValue) => void,
    ) => React.ReactNode;
}

export function EntryEditor({
    node,
    initialValue,
    errors,
    onSave,
    onClose,
    renderNode,
}: EntryEditorProps) {
    const [localValue, setLocalValue] = useState<JsonValue>(initialValue);

    const handleChange = (changedPointer: string, newValue: JsonValue) => {
        setLocalValue((previous) =>
            applyLocalChange(node.pointer, previous, changedPointer, newValue)
        );
    };

    return (
        <div className="space-y-4">
            {renderNode(node, localValue, errors, handleChange)}
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
 * Applies a local change within the editor's scope
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
