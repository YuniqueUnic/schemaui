import { type Dispatch, type SetStateAction, useMemo, useState } from "react";
import { AlertCircle, ChevronDown, ChevronRight, File } from "lucide-react";
import type { UiAst, UiNode } from "../types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

interface TreeViewProps {
  ast?: UiAst | null;
  selectedPointer?: string;
  errors?: Map<string, string>;
  onSelect(pointer: string): void;
}

interface TreeItem {
  pointer: string;
  label: string;
  depth: number;
  hasChildren: boolean;
  children: TreeItem[];
}

export function TreeView(
  { ast, selectedPointer, errors, onSelect }: TreeViewProps,
) {
  const items = useMemo(() => buildTree(ast?.roots ?? [], 0), [ast]);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  // Check if a pointer or any of its children have errors
  const hasError = (pointer: string): boolean => {
    if (errors?.has(pointer)) return true;
    // Check if any child path has an error
    for (const errorPath of errors?.keys() ?? []) {
      if (errorPath.startsWith(pointer + "/")) return true;
    }
    return false;
  };

  if (!items.length) {
    return (
      <div className="flex h-full items-center justify-center text-xs text-muted">
        No schema
      </div>
    );
  }

  return (
    <ScrollArea className="h-full">
      <div className="px-3 py-4 text-sm">
        {items.map((item) => (
          <TreeRow
            key={item.pointer}
            item={item}
            collapsed={collapsed}
            setCollapsed={setCollapsed}
            selectedPointer={selectedPointer}
            hasError={hasError}
            onSelect={onSelect}
          />
        ))}
      </div>
    </ScrollArea>
  );
}

function TreeRow({
  item,
  collapsed,
  setCollapsed,
  selectedPointer,
  hasError,
  onSelect,
}: {
  item: TreeItem;
  collapsed: Record<string, boolean>;
  setCollapsed: Dispatch<SetStateAction<Record<string, boolean>>>;
  selectedPointer?: string;
  hasError: (pointer: string) => boolean;
  onSelect(pointer: string): void;
}) {
  const isActive = selectedPointer === item.pointer ||
    selectedPointer?.startsWith(`${item.pointer}/`) === true;
  const isCollapsed = collapsed[item.pointer];
  const itemHasError = hasError(item.pointer);
  const toggle = (event: React.MouseEvent) => {
    event.stopPropagation();
    setCollapsed((prev) => ({ ...prev, [item.pointer]: !prev[item.pointer] }));
  };

  return (
    <div className="space-y-0.5">
      <button
        type="button"
        onClick={() => onSelect(item.pointer)}
        className={cn(
          "group flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors",
          "hover:bg-accent hover:text-accent-foreground",
          isActive && "bg-accent text-accent-foreground font-medium",
        )}
        style={{ paddingLeft: 8 + item.depth * 16 }}
      >
        {item.hasChildren
          ? (
            <span
              onClick={toggle}
              className="flex items-center justify-center"
            >
              {isCollapsed
                ? <ChevronRight className="h-4 w-4 text-muted-foreground" />
                : <ChevronDown className="h-4 w-4 text-muted-foreground" />}
            </span>
          )
          : <File className="h-4 w-4 text-muted-foreground" />}
        <span className="truncate text-left flex items-center gap-2">
          {item.label}
          {itemHasError && (
            <AlertCircle className="h-3.5 w-3.5 text-destructive flex-shrink-0" />
          )}
        </span>
      </button>
      {!isCollapsed &&
        item.children.map((child) => (
          <TreeRow
            key={child.pointer}
            item={child}
            collapsed={collapsed}
            setCollapsed={setCollapsed}
            selectedPointer={selectedPointer}
            hasError={hasError}
            onSelect={onSelect}
          />
        ))}
    </div>
  );
}

function buildTree(nodes: UiNode[], depth: number): TreeItem[] {
  return nodes.map((node) => {
    const label = node.title ?? pointerSegment(node.pointer) ?? "field";
    const children = node.kind.type === "object"
      ? buildTree(node.kind.children ?? [], depth + 1)
      : [];
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
  if (!pointer || pointer === "/") return undefined;
  const segments = pointer.split("/").filter(Boolean);
  return segments[segments.length - 1]?.replace(/~1/g, "/").replace(/~0/g, "~");
}
