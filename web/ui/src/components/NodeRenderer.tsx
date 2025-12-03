/**
 * NodeRenderer - Central dispatcher for UI AST nodes
 *
 * This component follows the Component Contract pattern from the refactor spec:
 * - Each UiNodeKind maps to a dedicated renderer component
 * - NodeRenderer handles only the chrome (header, errors) and dispatch
 * - Concrete rendering is delegated to FieldRenderer, ArrayRenderer, etc.
 */

import type { ReactNode } from "react";
import type { JsonValue, UiNode, UiNodeKind } from "../types";
import { ArrayRenderer } from "./renderers/ArrayRenderer";
import { CompositeRenderer } from "./renderers/CompositeRenderer";
import { FieldRenderer } from "./renderers/FieldRenderer";
import { ObjectRenderer } from "./renderers/ObjectRenderer";

// Type narrowing helpers
type FieldNode = UiNode & { kind: Extract<UiNodeKind, { type: "field" }> };
type ArrayNode = UiNode & { kind: Extract<UiNodeKind, { type: "array" }> };
type CompositeNode = UiNode & {
  kind: Extract<UiNodeKind, { type: "composite" }>;
};
type ObjectNode = UiNode & { kind: Extract<UiNodeKind, { type: "object" }> };

export interface NodeRendererProps {
  node: UiNode;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  renderMode?: "stack" | "inline";
}

/**
 * Main NodeRenderer component
 * Provides chrome (header, error display) and dispatches to specific renderers
 */
export function NodeRenderer({
  node,
  value,
  errors,
  onChange,
  renderMode = "stack",
}: NodeRendererProps) {
  const error = errors.get(node.pointer);

  const chromeClass = renderMode === "inline"
    ? "space-y-2"
    : node.kind.type === "object"
    ? "space-y-3"
    : "space-y-2 pb-4";

  return (
    <div className={chromeClass}>
      <NodeHeader node={node} />
      <NodeBody
        node={node}
        value={value}
        errors={errors}
        onChange={onChange}
      />
      {error && (
        <p className="text-xs text-destructive bg-destructive/10 px-2 py-1 rounded-md">
          {error}
        </p>
      )}
    </div>
  );
}

/**
 * Renders the node header with title, required badge, and description
 */
function NodeHeader({ node }: { node: UiNode }) {
  return (
    <header className="space-y-0.5">
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium text-foreground">
          {node.title ?? node.pointer}
        </span>
        {node.required && (
          <span className="text-[10px] font-medium uppercase tracking-wider text-destructive">
            required
          </span>
        )}
      </div>
      {node.description && (
        <p className="text-xs text-muted-foreground leading-relaxed">
          {node.description}
        </p>
      )}
    </header>
  );
}

/**
 * Dispatches rendering to the appropriate component based on node kind
 */
function NodeBody({
  node,
  value,
  errors,
  onChange,
}: Omit<NodeRendererProps, "renderMode">): ReactNode {
  // Recursive renderNode function for nested components
  const renderNode = (
    n: UiNode,
    v: JsonValue | undefined,
    e: Map<string, string>,
    oc: (pointer: string, value: JsonValue) => void,
  ) => (
    <NodeRenderer
      node={n}
      value={v}
      errors={e}
      onChange={oc}
      renderMode="inline"
    />
  );

  switch (node.kind.type) {
    case "field":
      return (
        <FieldRenderer
          node={node as FieldNode}
          value={value}
          onChange={onChange}
        />
      );

    case "array":
      return (
        <ArrayRenderer
          node={node as ArrayNode}
          value={value}
          errors={errors}
          onChange={onChange}
          renderNode={renderNode}
        />
      );

    case "composite":
      return (
        <CompositeRenderer
          node={node as CompositeNode}
          value={value}
          errors={errors}
          onChange={onChange}
          renderNode={renderNode}
        />
      );

    case "object":
      return (
        <ObjectRenderer
          node={node as ObjectNode}
          value={value}
          errors={errors}
          onChange={onChange}
          renderNode={renderNode}
        />
      );

    default:
      return null;
  }
}
