import { useMemo } from 'react';
import type { UiAst, UiNode } from '../types';

interface TreeViewProps {
  ast?: UiAst | null;
  selectedPointer?: string;
  onSelect(pointer: string): void;
}

interface TreeItem {
  pointer: string;
  label: string;
  depth: number;
  children: TreeItem[];
}

export function TreeView({ ast, selectedPointer, onSelect }: TreeViewProps) {
  const items = useMemo(() => buildTree(ast?.roots ?? []), [ast]);

  if (!ast || !ast.roots.length) {
    return (
      <div className="flex h-full items-center justify-center text-xs text-slate-400">
        No schema loaded
      </div>
    );
  }

  return (
    <div className="space-y-2 px-3 py-4">
      {items.map((item) => (
        <TreeNode key={item.pointer} item={item} selected={selectedPointer} onSelect={onSelect} />
      ))}
    </div>
  );
}

function TreeNode({
  item,
  selected,
  onSelect,
}: {
  item: TreeItem;
  selected?: string;
  onSelect(pointer: string): void;
}) {
  const isSelected = item.pointer === selected;
  return (
    <div>
      <button
        type="button"
        onClick={() => onSelect(item.pointer)}
        className={`flex w-full items-center rounded-lg px-3 py-2 text-left text-sm transition hover:bg-white/5 ${
          isSelected ? 'bg-sky-500/20 text-sky-300 ring-1 ring-sky-400/40' : 'text-slate-200'
        }`}
        style={{ paddingLeft: 12 + item.depth * 16 }}
      >
        <span className="truncate">{item.label}</span>
      </button>
      {item.children.map((child) => (
        <TreeNode key={child.pointer} item={child} selected={selected} onSelect={onSelect} />
      ))}
    </div>
  );
}

function buildTree(nodes: UiNode[], depth = 0): TreeItem[] {
  return nodes.map((node) => {
    const label = node.title ?? pointerSegment(node.pointer) ?? 'Field';
    let children: TreeItem[] = [];
    if (node.kind.type === 'object') {
      children = buildTree(node.kind.children ?? [], depth + 1);
    }
    return {
      pointer: node.pointer,
      label,
      depth,
      children,
    };
  });
}

function pointerSegment(pointer: string): string | undefined {
  if (!pointer || pointer === '/') return undefined;
  const segments = pointer.split('/').filter(Boolean);
  const last = segments[segments.length - 1];
  return last?.replace(/~1/g, '/').replace(/~0/g, '~');
}
