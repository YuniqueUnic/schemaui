import type { JsonValue, UiNode } from "../../types";
import { extractChildValue } from "../../utils/typeHelpers";
import { isNodeVisible } from "../../utils/visibility";

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
 * Renders object type nodes by recursively rendering their children.
 *
 * Children hidden by an `x-visible-when` rule are dropped here rather than in
 * the caller: this is the one place that knows which object the children belong
 * to, so it is also the only place that can honour a rule for an entry opened
 * from an array or a composite variant (those nodes are shared by every entry
 * and cannot be pruned once, up front).
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
      {(node.kind.children ?? [])
        .filter((child) => isNodeVisible(child, value))
        .map((child) => (
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
