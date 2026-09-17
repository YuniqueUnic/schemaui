import type { JsonValue, UiAst, UiNode } from "../types";
import { deepEqual } from "./deepEqual";
import { extractChildValue } from "./typeHelpers";

/**
 * Drops the nodes whose `visible_when` rule the current data does not satisfy.
 *
 * Visibility is a property of the containing object: a rule names a sibling
 * property, so it is evaluated against the object that owns the node. Pruning
 * the AST once, up front, keeps every view (tree, layout explorer, editor,
 * breadcrumbs) consistent without teaching each renderer about visibility.
 */
export function pruneHiddenNodes(
  ast: UiAst | undefined | null,
  data: JsonValue,
): UiAst | undefined {
  if (!ast) return undefined;
  return { ...ast, roots: pruneNodes(ast.roots, data) };
}

function pruneNodes(nodes: UiNode[], container: JsonValue | undefined): UiNode[] {
  return nodes
    .filter((node) => isNodeVisible(node, container))
    .map((node) => pruneNode(node, container));
}

function pruneNode(node: UiNode, container: JsonValue | undefined): UiNode {
  if (node.kind.type !== "object") return node;
  const children = pruneNodes(
    node.kind.children ?? [],
    extractChildValue(container, node.pointer),
  );
  return { ...node, kind: { ...node.kind, children } };
}

/**
 * Whether a node's rule holds for the object that contains it.
 *
 * Pruning covers the nodes the AST owns outright. Nodes reached through an
 * array item or a composite variant are shared by every entry, so they cannot
 * be pruned once — the renderer that owns the entry evaluates the same rule
 * against that entry's value instead.
 */
export function isNodeVisible(
  node: UiNode,
  container: JsonValue | undefined,
): boolean {
  const rule = node.visible_when;
  if (!rule) return true;

  const actual = readProperty(container, rule.field);
  if (rule.op === "contains") {
    return Array.isArray(actual) &&
      actual.some((entry) => deepEqual(entry, rule.value));
  }
  return deepEqual(actual, rule.value);
}

/** Reads a sibling property, treating anything but an object as an empty one. */
function readProperty(
  container: JsonValue | undefined,
  field: string,
): JsonValue | undefined {
  if (
    container === null || container === undefined ||
    typeof container !== "object" || Array.isArray(container)
  ) {
    return undefined;
  }
  return (container as Record<string, JsonValue>)[field];
}
