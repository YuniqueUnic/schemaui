import { useMemo } from "react";
import type { UiAst, UiLayout, UiNode } from "../types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

interface LayoutExplorerProps {
    layout?: UiLayout | null;
    ast?: UiAst | null;
    selectedPointer?: string;
    onSelect(pointer: string): void;
}

interface LayoutItem {
    id: string;
    label: string;
    depth: number;
    pointer?: string;
    kind: "section" | "field";
}

export function LayoutExplorer(
    { layout, ast, selectedPointer, onSelect }: LayoutExplorerProps,
) {
    const labelMap = useMemo(() => buildPointerLabelMap(ast), [ast]);

    if (!layout || layout.roots.length === 0) {
        return (
            <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
                No layout available
            </div>
        );
    }

    const items: LayoutItem[] = [];

    for (const root of layout.roots) {
        for (const section of root.sections) {
            items.push(...buildItemsForSection(section, 0, labelMap));
        }
    }

    return (
        <ScrollArea className="h-full">
            <div className="px-3 py-4 text-sm space-y-0.5">
                {items.map((item) => {
                    const isActive = item.pointer &&
                        item.pointer === selectedPointer;
                    return (
                        <button
                            key={item.id}
                            type="button"
                            onClick={() =>
                                item.pointer && onSelect(item.pointer)}
                            className={cn(
                                "group flex w-full items-center rounded-md px-2 py-1.5 text-sm transition-colors text-left",
                                "hover:bg-accent hover:text-accent-foreground",
                                isActive &&
                                    "bg-accent text-accent-foreground font-medium",
                                !item.pointer && "cursor-default opacity-80",
                            )}
                            style={{ paddingLeft: 8 + item.depth * 16 }}
                        >
                            <span className="truncate">
                                {item.label}
                            </span>
                        </button>
                    );
                })}
            </div>
        </ScrollArea>
    );
}

function buildItemsForSection(
    section: UiLayout["roots"][number]["sections"][number],
    depth: number,
    labelMap: Map<string, string>,
): LayoutItem[] {
    const items: LayoutItem[] = [];

    items.push({
        id: section.id,
        label: section.title,
        depth,
        pointer: section.pointer || undefined,
        kind: "section",
    });

    for (const fp of section.field_pointers) {
        items.push({
            id: `${section.id}::field::${fp}`,
            label: labelMap.get(fp) ?? pointerSegment(fp) ?? fp,
            depth: depth + 1,
            pointer: fp,
            kind: "field",
        });
    }

    for (const child of section.children) {
        items.push(...buildItemsForSection(child, depth + 1, labelMap));
    }

    return items;
}

function buildPointerLabelMap(ast?: UiAst | null): Map<string, string> {
    const map = new Map<string, string>();
    if (!ast) return map;

    for (const root of ast.roots) {
        visitNode(root, map);
    }

    return map;
}

function visitNode(node: UiNode, map: Map<string, string>) {
    const pointer = node.pointer;
    const title = node.title ?? pointerSegment(pointer) ?? pointer;
    if (pointer) {
        map.set(pointer, title);
    }

    if (node.kind.type === "object") {
        for (const child of node.kind.children ?? []) {
            visitNode(child, map);
        }
    }
}

function pointerSegment(pointer: string): string | undefined {
    if (!pointer || pointer === "/") return undefined;
    const segments = pointer.split("/").filter(Boolean);
    return segments[segments.length - 1]?.replace(/~1/g, "/").replace(
        /~0/g,
        "~",
    );
}
