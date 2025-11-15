import { memo } from "react";
import { clsx } from "clsx";
import { ChevronRight, FolderTree, Layers, Search } from "lucide-react";
import type { SectionPath, TreeNode } from "../utils/blueprint";

interface TreePanelProps {
  nodes: TreeNode<SectionPath>[];
  expanded: Set<string>;
  activeId?: string;
  onToggle: (nodeId: string) => void;
  onSelect: (path: SectionPath, nodeId: string) => void;
  filter: string;
  onFilterChange: (value: string) => void;
  errorCounts: Map<string, number>;
  loading?: boolean;
}

export const TreePanel = memo(function TreePanel({
  nodes,
  expanded,
  activeId,
  onToggle,
  onSelect,
  filter,
  onFilterChange,
  errorCounts,
  loading = false,
}: TreePanelProps) {
  if (loading) {
    return (
      <aside className="flex h-full flex-col gap-3 border-r border-slate-200 bg-white p-6 dark:border-slate-800/70 dark:bg-slate-950/80">
        <div className="h-10 animate-pulse rounded-2xl bg-slate-800/60" />
        <div className="space-y-3">
          {Array.from({ length: 6 }).map((_, index) => (
            <div
              key={index}
              className="h-9 animate-pulse rounded-xl bg-slate-800/60"
            />
          ))}
        </div>
      </aside>
    );
  }

  return (
    <aside className="flex h-full w-full flex-col border-r border-slate-200 bg-white dark:border-slate-800/70 dark:bg-slate-950/80">
      <label className="group relative mx-4 mt-4 flex items-center rounded-xl border border-slate-200 bg-white px-3 py-2 shadow-sm transition dark:border-slate-800/70 dark:bg-slate-900/60">
        <Search className="h-4 w-4 text-slate-500 transition group-focus-within:text-brand-500 dark:text-slate-500 dark:group-focus-within:text-brand-300" />
        <input
          value={filter}
          onChange={(event) => onFilterChange(event.target.value)}
          placeholder="Search sections"
          className="ml-2 flex-1 border-none bg-transparent text-sm text-slate-700 placeholder:text-slate-400 focus:outline-none dark:text-slate-200 dark:placeholder:text-slate-500"
        />
      </label>
      <div className="mt-3 flex-1 overflow-auto px-2 pb-6">
        {nodes.map((node) => (
          <TreeNodeItem
            key={node.id}
            node={node}
            expanded={expanded}
            activeId={activeId}
            onToggle={onToggle}
            onSelect={onSelect}
            filter={filter}
            errorCounts={errorCounts}
          />
        ))}
      </div>
    </aside>
  );
});

interface TreeNodeItemProps {
  node: TreeNode<SectionPath>;
  expanded: Set<string>;
  activeId?: string;
  onToggle: (id: string) => void;
  onSelect: (path: SectionPath, nodeId: string) => void;
  filter: string;
  errorCounts: Map<string, number>;
}

function TreeNodeItem({
  node,
  expanded,
  activeId,
  onToggle,
  onSelect,
  filter,
  errorCounts,
}: TreeNodeItemProps) {
  const normalizedFilter = filter.trim().toLowerCase();
  const matchesFilter = !normalizedFilter ||
    node.label.toLowerCase().includes(normalizedFilter) ||
    (node.description ?? "").toLowerCase().includes(normalizedFilter);
  const anyChildVisible = node.children.length > 0 &&
    node.children.some((child) =>
      nodeVisible(child, expanded, normalizedFilter)
    );
  if (!matchesFilter && !anyChildVisible) {
    return null;
  }
  const isExpanded = expanded.has(node.id) || Boolean(normalizedFilter);
  const isLeaf = node.children.length === 0;
  const nodeErrorCount = errorCounts.get(node.id) ?? 0;
  const Icon = node.depth === 0 ? Layers : FolderTree;
  return (
    <div className="text-sm">
      <button
        type="button"
        onClick={() => onSelect(node.data, node.id)}
        className={clsx(
          "group flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-slate-700 transition hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800/60",
          activeId === node.id &&
            "bg-gradient-to-r from-brand-500/10 via-brand-500/5 to-transparent text-brand-700 dark:text-brand-100",
        )}
        style={{ paddingLeft: `${node.depth * 0.9 + 0.5}rem` }}
      >
        {!isLeaf
          ? (
            <span
              onClick={(event) => {
                event.stopPropagation();
                onToggle(node.id);
              }}
              className={clsx(
                "mr-2 flex h-6 w-6 items-center justify-center rounded-full border border-slate-300 text-slate-500 transition hover:border-brand-400 hover:text-brand-500 dark:border-slate-700 dark:text-slate-400 dark:hover:text-brand-300",
              )}
            >
              <ChevronRight
                className={clsx(
                  "h-3.5 w-3.5 transition-transform",
                  (isExpanded || normalizedFilter) && "rotate-90",
                )}
              />
            </span>
          )
          : <span className="mr-2 h-6 w-6" />}
        <Icon className="h-3.5 w-3.5 text-slate-400 group-hover:text-brand-500 dark:text-slate-500 dark:group-hover:text-brand-300" />
        <span className="flex-1 truncate text-sm">
          {highlightMatch(node.label, normalizedFilter)}
        </span>
        {nodeErrorCount > 0
          ? (
            <span className="rounded-full bg-rose-500/15 px-2 py-0.5 text-[10px] font-semibold text-rose-500 dark:text-rose-200">
              {nodeErrorCount}
            </span>
          )
          : null}
      </button>
      {isExpanded && node.children.length > 0
        ? (
          <div className="space-y-0.5">
            {node.children.map((child) => (
              <TreeNodeItem
                key={child.id}
                node={child}
                expanded={expanded}
                activeId={activeId}
                onToggle={onToggle}
                onSelect={onSelect}
                filter={filter}
                errorCounts={errorCounts}
              />
            ))}
          </div>
        )
        : null}
    </div>
  );
}

function nodeVisible(
  node: TreeNode<SectionPath>,
  expanded: Set<string>,
  filter: string,
): boolean {
  const matches = !filter ||
    node.label.toLowerCase().includes(filter) ||
    (node.description ?? "").toLowerCase().includes(filter);
  if (matches) {
    return true;
  }
  if (node.children.length === 0) {
    return false;
  }
  return node.children.some((child) => nodeVisible(child, expanded, filter));
}

function highlightMatch(text: string, query: string) {
  if (!query) {
    return text;
  }
  const index = text.toLowerCase().indexOf(query);
  if (index === -1) {
    return text;
  }
  return (
    <>
      {text.slice(0, index)}
      <span className="rounded bg-brand-400/20 px-0.5 text-brand-200">
        {text.slice(index, index + query.length)}
      </span>
      {text.slice(index + query.length)}
    </>
  );
}
