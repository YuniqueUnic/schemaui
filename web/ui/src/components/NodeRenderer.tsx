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
import { KeyValueRenderer } from "./renderers/KeyValueRenderer";
import { ObjectRenderer } from "./renderers/ObjectRenderer";
import { RangeControl } from "./renderers/controls/RangeControl";
import { RichSurface } from "./rich/RichSurface";
import { resolveControl } from "../lib/control";

// Type narrowing helpers
type FieldNode = UiNode & { kind: Extract<UiNodeKind, { type: "field" }> };
type ArrayNode = UiNode & { kind: Extract<UiNodeKind, { type: "array" }> };
type KeyValueNode = UiNode & {
  kind: Extract<UiNodeKind, { type: "key_value" }>;
};
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
  /**
   * Render the control only, without the title/description block.
   *
   * The editor already shows the selected node's title and description above the
   * card, so a leaf card that repeated them would state the same heading twice
   * in two different styles. Object children keep their headers: each card needs
   * its own label to be readable.
   */
  hideHeader?: boolean;
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
  hideHeader = false,
}: NodeRendererProps) {
  const error = errors.get(node.pointer);

  // Spacing is owned by the caller: cards already carry their own padding, and
  // a trailing `pb-4` here used to stack on top of it, making every card taller
  // than it looked.
  const chromeClass = renderMode === "inline"
    ? "space-y-2"
    : node.kind.type === "object"
    ? "space-y-3"
    : "space-y-1.5";

  return (
    <div className={chromeClass}>
      {!hideHeader && <NodeHeader node={node} />}
      {node.content && (
        <RichSurface
          content={node.content}
          variant="block"
          title={node.title ?? undefined}
        />
      )}
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
  const title = node.title?.trim();
  return (
    <header className="space-y-1">
      <div className="flex items-center gap-2">
        <span
          className={
            title
              ? "text-[13px] font-medium leading-5 text-foreground"
              : "font-mono text-xs leading-5 text-muted-foreground"
          }
        >
          {title || node.pointer}
        </span>
        {node.required && <RequiredTag />}
      </div>
      {node.description && (
        <p className="text-xs text-muted-foreground leading-relaxed">
          {node.description}
        </p>
      )}
    </header>
  );
}

/** The one way this app marks a field as required. */
export function RequiredTag() {
  return (
    <span className="shrink-0 rounded border border-destructive/25 bg-destructive/8 px-1.5 py-px text-[10px] font-medium leading-4 text-destructive">
      Required
    </span>
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
      // A two-number array that asked for `range` is an interval, not a list.
      // Checked before the list renderer so the schema's intent wins.
      if (resolveControl(node) === "range") {
        return (
          <RangeControl
            node={node}
            value={value}
            onChange={onChange}
          />
        );
      }
      return (
        <ArrayRenderer
          node={node as ArrayNode}
          value={value}
          errors={errors}
          onChange={onChange}
          renderNode={renderNode}
        />
      );

    case "key_value":
      return (
        <KeyValueRenderer
          node={node as KeyValueNode}
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
