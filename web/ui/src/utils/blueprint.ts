import type { WebBlueprint, WebField, WebSection } from '../types';

export interface SectionPath {
  rootIndex: number;
  sectionPath: number[];
}

export interface TreeNode<T = unknown> {
  id: string;
  depth: number;
  label: string;
  description?: string | null;
  data: T;
  children: TreeNode<T>[];
}

export function buildSectionTree(blueprint?: WebBlueprint): TreeNode<SectionPath>[] {
  if (!blueprint) return [];
  return blueprint.roots.map((root, rootIndex) => ({
    id: root.id,
    depth: 0,
    label: root.title || `Root ${rootIndex + 1}`,
    description: root.description,
    data: { rootIndex, sectionPath: [] },
    children: (root.sections || []).map((section, index) =>
      mapSection(section, rootIndex, [index], 1),
    ),
  }));
}

function mapSection(
  section: WebSection,
  rootIndex: number,
  path: number[],
  depth: number,
): TreeNode<SectionPath> {
  return {
    id: section.id,
    depth,
    label: section.title || `Section ${path[path.length - 1] + 1}`,
    description: section.description,
    data: { rootIndex, sectionPath: path },
    children: (section.sections || []).map((child, idx) =>
      mapSection(child, rootIndex, [...path, idx], depth + 1),
    ),
  };
}

export function getSectionByPath(
  blueprint: WebBlueprint | undefined,
  target: SectionPath,
): WebSection | undefined {
  if (!blueprint) return undefined;
  const root = blueprint.roots[target.rootIndex];
  if (!root) return undefined;
  if (target.sectionPath.length === 0) {
    return {
      id: root.id,
      title: root.title,
      description: root.description,
      fields: [],
      sections: root.sections ?? [],
    };
  }
  let current: WebSection | undefined;
  let sections = root.sections || [];
  for (const index of target.sectionPath) {
    current = sections?.[index];
    sections = current?.sections || [];
  }
  return current;
}

export function collectFields(section?: WebSection): WebField[] {
  return section?.fields ?? [];
}
