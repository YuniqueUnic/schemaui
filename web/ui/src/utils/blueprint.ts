import type { UiAst, UiSection } from "../ui-ast";

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
  fieldPointers: string[];
  children: TreeNode<T>[];
}

export function buildSectionTree(
  ast?: UiAst,
): TreeNode<SectionPath>[] {
  if (!ast) return [];
  const nodes: TreeNode<SectionPath>[] = [];
  ast.roots.forEach((root, rootIndex) => {
    const sections = root.sections || [];
    if (sections.length === 1) {
      const collapsed: UiSection = {
        ...sections[0],
        title: sections[0].title || root.title,
        description: sections[0].description ?? root.description,
      };
      nodes.push(mapSection(collapsed, rootIndex, [0], 0));
      return;
    }
    nodes.push({
      id: root.id || `root-${rootIndex}`,
      depth: 0,
      label: root.title || `Root ${rootIndex + 1}`,
      description: root.description,
      data: { rootIndex, sectionPath: [] },
      fieldPointers: [],
      children: sections.map((section, index) =>
        mapSection(section, rootIndex, [index], 1)
      ),
    });
  });
  return nodes;
}

function mapSection(
  section: UiSection,
  rootIndex: number,
  path: number[],
  depth: number,
): TreeNode<SectionPath> {
  return {
    id: section.id || `${rootIndex}-${path.join("-")}`,
    depth,
    label: section.title || `Section ${path[path.length - 1] + 1}`,
    description: section.description,
    data: { rootIndex, sectionPath: path },
    fieldPointers: (section.children || [])
      .map((node) => node.pointer)
      .filter((pointer): pointer is string => Boolean(pointer)),
    children: (section.sections || []).map((child, idx) =>
      mapSection(child, rootIndex, [...path, idx], depth + 1)
    ),
  };
}

export function getSectionByPath(
  ast: UiAst | undefined,
  target: SectionPath,
): UiSection | undefined {
  if (!ast) return undefined;
  const root = ast.roots[target.rootIndex];
  if (!root) return undefined;
  if (target.sectionPath.length === 0) {
    const soleChild = root.sections?.length === 1 ? root.sections[0] : undefined;
    if (soleChild && (soleChild.children?.length ?? 0) > 0) {
      return soleChild;
    }
    return {
      id: root.id,
      title: root.title,
      description: root.description,
      children: [],
      sections: root.sections ?? [],
    };
  }
  let current: UiSection | undefined;
  let sections = root.sections || [];
  for (const index of target.sectionPath) {
    current = sections?.[index];
    sections = current?.sections || [];
  }
  return current;
}

export function getBreadcrumbs(
  ast: UiAst | undefined,
  target: SectionPath,
): string[] {
  if (!ast) {
    return [];
  }
  const crumbs: string[] = [];
  const root = ast.roots[target.rootIndex];
  if (!root) {
    return crumbs;
  }
  crumbs.push(root.title || "Root");
  let sections = root.sections || [];
  target.sectionPath.forEach((index) => {
    const section = sections?.[index];
    if (section) {
      crumbs.push(section.title || `Section ${index + 1}`);
      sections = section.sections || [];
    }
  });
  return crumbs;
}
