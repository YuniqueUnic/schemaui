import type { JsonValue, UiNode } from "../../types";
import { extractChildValue } from "../../utils/typeHelpers";

interface ObjectRendererProps {
    node: UiNode;
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
 * Renders object type nodes by recursively rendering their children
 */
export function ObjectRenderer({
    node,
    value,
    errors,
    onChange,
    renderNode,
}: ObjectRendererProps) {
    if (node.kind.type !== "object") {
        return null;
    }

    return (
        <div className="space-y-4">
            {(node.kind.children ?? []).map((child) => (
                <div key={child.pointer}>
                    {renderNode(
                        child,
                        extractChildValue(value, child.pointer),
                        errors,
                        onChange,
                    )}
                </div>
            ))}
        </div>
    );
}
