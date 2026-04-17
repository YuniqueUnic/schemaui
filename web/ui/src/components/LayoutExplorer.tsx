import { useMemo } from "react";
import { Braces, Hash, ToggleLeft, Type as TypeIcon } from "lucide-react";
import type {
  ScalarKind,
  UiAst,
  UiLayout,
  UiNode,
  UiNodeKind,
} from "../types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

interface LayoutExplorerProps {
    layout?: UiLayout | null;
    ast?: UiAst | null;
    selectedPointer?: string;
    onSelect(pointer: string): void;
    rootLabel?: string;
}

interface LayoutItem {
    id: string;
    label: string;
    depth: number;
    pointer: string;
    kind: "section" | "field";
    selectable: boolean;
    scalar?: ScalarKind;
}

export function LayoutExplorer(
    {
        layout,
        ast,
        selectedPointer,
        onSelect,
        rootLabel,
    }: LayoutExplorerProps,
) {
    const items = useMemo(() => {
        if (!ast || ast.roots.length === 0) return [];
        const resolvedRootLabel = rootLabel ??
            layout?.roots?.[0]?.title ?? "General";
        return buildItems(ast, resolvedRootLabel);
    }, [ast, layout, rootLabel]);

    if (items.length === 0) {
        return (
            <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
                No layout available
            </div>
        );
    }

    return (
        <ScrollArea className="h-full">
            <div className="px-2 py-3 text-sm space-y-0.5">
                {items.map((item) => {
                    const isActive = item.selectable &&
                        item.pointer === selectedPointer;
                    return (
                        <button
                            key={item.id}
                            type="button"
                            onClick={() =>
                                item.selectable && onSelect(item.pointer)}
                            className={cn(
                                "group flex w-full items-center gap-2 rounded-md py-1.5 pr-2 text-sm text-left transition-colors",
                                "hover:bg-accent hover:text-accent-foreground",
                                isActive &&
                                    "bg-accent text-accent-foreground font-medium",
                                !item.selectable && "cursor-default opacity-80",
                            )}
                            style={{ paddingLeft: 8 + item.depth * 14 }}
                        >
                            <ItemIcon item={item} />
                            <span className="truncate">{item.label}</span>
                        </button>
                    );
                })}
            </div>
        </ScrollArea>
    );
}

function ItemIcon({ item }: { item: LayoutItem }) {
    if (item.kind === "section") {
        return <Braces className="h-3.5 w-3.5 text-muted-foreground shrink-0" />;
    }
    switch (item.scalar) {
        case "boolean":
            return (
                <ToggleLeft className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            );
        case "integer":
        case "number":
            return <Hash className="h-3.5 w-3.5 text-muted-foreground shrink-0" />;
        default:
            return (
                <TypeIcon className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            );
    }
}

function buildItems(ast: UiAst, rootLabel: string): LayoutItem[] {
    const items: LayoutItem[] = [];
    items.push({
        id: "section:root",
        label: rootLabel,
        depth: 0,
        pointer: "",
        kind: "section",
        selectable: true,
    });
    for (const node of ast.roots) {
        collectItems(node, 1, items);
    }
    return items;
}

function collectItems(node: UiNode, depth: number, out: LayoutItem[]): void {
    const label = node.title ?? pointerSegment(node.pointer) ?? node.pointer;
    if (node.kind.type === "object") {
        out.push({
            id: `section:${node.pointer}`,
            label,
            depth,
            pointer: node.pointer,
            kind: "section",
            selectable: true,
        });
        for (const child of node.kind.children ?? []) {
            collectItems(child, depth + 1, out);
        }
        return;
    }
    out.push({
        id: `field:${node.pointer}`,
        label,
        depth,
        pointer: node.pointer,
        kind: "field",
        selectable: true,
        scalar: inferScalar(node.kind),
    });
}

function inferScalar(kind: UiNodeKind): ScalarKind | undefined {
    if (kind.type === "field") return kind.scalar;
    return undefined;
}

function pointerSegment(pointer: string): string | undefined {
    if (!pointer || pointer === "/") return undefined;
    const segments = pointer.split("/").filter(Boolean);
    return segments[segments.length - 1]?.replace(/~1/g, "/").replace(
        /~0/g,
        "~",
    );
}
