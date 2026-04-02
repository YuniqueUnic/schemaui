import type { UiNode, UiNodeKind } from "../types";
import { joinPointer, splitPointer } from "./jsonPointer";
import { materializeCompositeKind } from "./schemaToUiKind";

type ArrayNode = UiNode & { kind: Extract<UiNodeKind, { type: "array" }> };

export function findNodeByPointer(
  nodes: UiNode[],
  pointer?: string,
): UiNode | undefined {
  if (!pointer) {
    return nodes[0];
  }

  let candidate: string | undefined = pointer;
  while (candidate) {
    const exact = findExactStaticNode(nodes, candidate) ??
      findExactArrayEntryNode(nodes, candidate);
    if (exact) {
      return exact;
    }
    candidate = parentPointer(candidate);
  }

  return undefined;
}

export function resolveNavigablePointer(
  nodes: UiNode[],
  pointer: string,
): string {
  return findNodeByPointer(nodes, pointer)?.pointer ?? pointer;
}

function findExactStaticNode(
  nodes: UiNode[],
  pointer: string,
): UiNode | undefined {
  for (const node of nodes) {
    if (node.pointer === pointer) {
      return node;
    }
    if (node.kind.type === "object") {
      const child = findExactStaticNode(node.kind.children ?? [], pointer);
      if (child) {
        return child;
      }
    }
  }
  return undefined;
}

function findExactArrayEntryNode(
  nodes: UiNode[],
  pointer: string,
): UiNode | undefined {
  for (const node of nodes) {
    const entry = findArrayEntryNodeInTree(node, pointer);
    if (entry) {
      return entry;
    }
  }
  return undefined;
}

function findArrayEntryNodeInTree(
  node: UiNode,
  pointer: string,
): UiNode | undefined {
  if (node.kind.type === "array") {
    const index = parseArrayEntryIndex(node.pointer, pointer);
    if (index !== undefined) {
      return createArrayEntryNode(node as ArrayNode, index);
    }
  }

  if (node.kind.type === "object") {
    for (const child of node.kind.children ?? []) {
      const entry = findArrayEntryNodeInTree(child, pointer);
      if (entry) {
        return entry;
      }
    }
  }

  return undefined;
}

function createArrayEntryNode(node: ArrayNode, index: number): UiNode {
  return {
    pointer: `${node.pointer}/${index}`,
    title: node.title
      ? `${node.title} entry ${index + 1}`
      : `Entry ${index + 1}`,
    description: node.description,
    required: false,
    default_value: node.default_value,
    kind: materializeCompositeKind(node.kind.item),
  };
}

function parseArrayEntryIndex(
  arrayPointer: string,
  pointer: string,
): number | undefined {
  const arraySegments = splitPointer(arrayPointer);
  const pointerSegments = splitPointer(pointer);

  if (pointerSegments.length !== arraySegments.length + 1) {
    return undefined;
  }

  const sharedPrefix = arraySegments.every((segment, index) =>
    pointerSegments[index] === segment
  );
  if (!sharedPrefix) {
    return undefined;
  }

  const entrySegment = pointerSegments[pointerSegments.length - 1];
  const index = Number(entrySegment);
  return Number.isInteger(index) && index >= 0 ? index : undefined;
}

function parentPointer(pointer: string): string | undefined {
  const segments = splitPointer(pointer);
  if (segments.length <= 1) {
    return undefined;
  }
  return joinPointer(segments.slice(0, -1));
}
