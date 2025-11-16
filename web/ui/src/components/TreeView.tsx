import { useMemo, useState, type Dispatch, type SetStateAction } from 'react';
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
  hasChildren: boolean;
  children: TreeItem[];
}

export function TreeView({ ast, selectedPointer, onSelect }: TreeViewProps) {
  const items = useMemo(() => buildTree(ast?.roots ?? [], 0), [ast]);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  if (!items.length) {
    return (
      <div className="flex h-full items-center justify-center text-xs text-muted">
        No schema
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto px-3 py-4 text-sm text-primary">
      {items.map((item) => (
        <TreeRow
          key={item.pointer}
          item={item}
          collapsed={collapsed}
          setCollapsed={setCollapsed}
          selectedPointer={selectedPointer}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

function TreeRow({
  item,
  collapsed,
  setCollapsed,
  selectedPointer,
  onSelect,
}: {
  item: TreeItem;
  collapsed: Record<string, boolean>;
  setCollapsed: Dispatch<SetStateAction<Record<string, boolean>>>;
  selectedPointer?: string;
  onSelect(pointer: string): void;
}) {
  const isActive = item.pointer === selectedPointer;
  const isCollapsed = collapsed[item.pointer];
  const toggle = (event: React.MouseEvent) => {
    event.stopPropagation();
    setCollapsed((prev) => ({ ...prev, [item.pointer]: !prev[item.pointer] }));
  };

  return (
    <div className="space-y-1">
      <button
        type="button"
        onClick={() => onSelect(item.pointer)}
        className={`group flex w-full items-center gap-2 rounded-lg px-2 py-1 transition hover:bg-[var(--app-panel-muted)] ${
          isActive ? 'bg-[var(--app-panel-muted)] text-[var(--app-accent)]' : 'text-primary'
        }`}
        style={{ paddingLeft: 8 + item.depth * 12 }}
      >
        {item.hasChildren ? (
          <span
            onClick={toggle}
            className="inline-flex h-4 w-4 items-center justify-center rounded border border-theme bg-panel text-[10px] text-muted group-hover:border-theme"
          >
            {isCollapsed ? '+' : '–'}
          </span>
        ) : (
          <span className="inline-block h-4 w-4" />
        )}
        <span className="truncate text-left">{item.label}</span>
      </button>
      {!isCollapsed &&
        item.children.map((child) => (
          <TreeRow
            key={child.pointer}
            item={child}
            collapsed={collapsed}
            setCollapsed={setCollapsed}
            selectedPointer={selectedPointer}
            onSelect={onSelect}
          />
        ))}
    </div>
  );
}

function buildTree(nodes: UiNode[], depth: number): TreeItem[] {
  return nodes.map((node) => {
    const label = node.title ?? pointerSegment(node.pointer) ?? 'field';
    const children = node.kind.type === 'object' ? buildTree(node.kind.children ?? [], depth + 1) : [];
    return {
      pointer: node.pointer,
      label,
      depth,
      hasChildren: children.length > 0,
      children,
    };
  });
}

function pointerSegment(pointer: string): string | undefined {
  if (!pointer || pointer === '/') return undefined;
  const segments = pointer.split('/').filter(Boolean);
  return segments[segments.length - 1]?.replace(/~1/g, '/').replace(/~0/g, '~');
}
