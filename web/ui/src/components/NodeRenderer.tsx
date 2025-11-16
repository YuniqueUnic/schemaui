import { useOverlay } from "./Overlay";
import { variantMatches } from "../utils/variantMatch";
import type { JsonValue, UiNode, UiNodeKind, UiVariant } from "../types";
import { defaultForKind, variantDefault } from "../ui-ast";
import type { ReactNode } from "react";

interface NodeRendererProps {
  node: UiNode;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  renderMode?: "stack" | "inline";
}

export function NodeRenderer(
  { node, value, errors, onChange, renderMode = "stack" }: NodeRendererProps,
) {
  const overlay = useOverlay();
  const error = errors.get(node.pointer);

  const chromeClass = renderMode === "inline"
    ? "space-y-3"
    : node.kind.type === "object"
    ? "space-y-4"
    : "space-y-3 border-b border-theme pb-4";

  return (
    <div className={chromeClass}>
      <header className="flex items-center justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-primary">
            {node.title ?? node.pointer}
            {node.required
              ? (
                <span className="ml-2 text-[10px] uppercase tracking-[0.3em] text-rose-400">
                  required
                </span>
              )
              : null}
          </p>
          {node.description
            ? <p className="text-xs text-muted">{node.description}</p>
            : null}
        </div>
      </header>
      {renderBody(node, value, errors, onChange, overlay)}
      {error ? <p className="text-xs text-rose-400">{error}</p> : null}
    </div>
  );
}

type FieldNode = UiNode & { kind: Extract<UiNodeKind, { type: "field" }> };
type ArrayNode = UiNode & { kind: Extract<UiNodeKind, { type: "array" }> };
type CompositeNode = UiNode & {
  kind: Extract<UiNodeKind, { type: "composite" }>;
};

function renderBody(
  node: UiNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
  overlay: ReturnType<typeof useOverlay>,
): ReactNode {
  switch (node.kind.type) {
    case "field":
      return renderFieldControl(node as FieldNode, value, onChange);
    case "array":
      return renderArrayControl(
        node as ArrayNode,
        value,
        errors,
        onChange,
        overlay,
      );
    case "composite":
      return renderCompositeControl(
        node as CompositeNode,
        value,
        errors,
        onChange,
        overlay,
      );
    case "object":
      return renderObjectControl(node, value, errors, onChange);
    default:
      return null;
  }
}

function renderObjectControl(
  node: UiNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
) {
  if (node.kind.type !== "object") {
    return null;
  }
  return (
    <div className="space-y-4">
      {(node.kind.children ?? []).map((child) => (
        <NodeRenderer
          key={child.pointer}
          node={child}
          value={extractChildValue(value, child.pointer)}
          errors={errors}
          onChange={onChange}
          renderMode="inline"
        />
      ))}
    </div>
  );
}

function renderFieldControl(
  node: FieldNode,
  value: JsonValue | undefined,
  onChange: (pointer: string, value: JsonValue) => void,
) {
  const resolved = value ?? node.default_value ?? defaultForKind(node.kind);
  if (node.kind.enum_options?.length) {
    return (
      <select
        className="w-full input-surface focus:border-[var(--app-accent)] focus:outline-none"
        value={(resolved as string) ?? ""}
        onChange={(event) => onChange(node.pointer, event.target.value)}
      >
        {node.kind.enum_options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    );
  }

  switch (node.kind.scalar) {
    case "integer":
    case "number":
      return (
        <input
          type="number"
          className="input-surface focus:border-[var(--app-accent)] focus:outline-none"
          value={typeof resolved === "number" ? resolved : 0}
          onChange={(event) =>
            onChange(node.pointer, Number(event.target.value))}
        />
      );
    case "boolean":
      return (
        <label className="inline-flex items-center gap-3 text-sm text-primary">
          <input
            type="checkbox"
            checked={Boolean(resolved)}
            onChange={(event) => onChange(node.pointer, event.target.checked)}
            className="h-4 w-4 rounded border-theme bg-panel text-[var(--app-accent)]"
          />
          Toggle
        </label>
      );
    case "string":
    default:
      return (
        <input
          type="text"
          className="input-surface focus:border-[var(--app-accent)] focus:outline-none"
          value={(resolved as string) ?? ""}
          onChange={(event) => onChange(node.pointer, event.target.value)}
        />
      );
  }
}

function renderArrayControl(
  node: ArrayNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
  overlay: ReturnType<typeof useOverlay>,
) {
  const entries = Array.isArray(value) ? (value as JsonValue[]) : [];

  const editEntry = (index: number, initial?: JsonValue) => {
    const entryNode: UiNode = {
      pointer: `${node.pointer}/${index}`,
      title: node.title
        ? `${node.title} entry ${index + 1}`
        : `Entry ${index + 1}`,
      description: node.description,
      required: false,
      default_value: node.default_value,
      kind: node.kind.item,
    };
    overlay.open({
      title: `${node.title ?? node.pointer} · Item ${index + 1}`,
      content: (close) => (
        <div className="space-y-4">
          <NodeRenderer
            node={entryNode}
            value={initial ?? entries[index]}
            errors={errors}
            onChange={(_pointer, newValue) => {
              const next = [...entries];
              next[index] = newValue;
              onChange(node.pointer, next);
            }}
            renderMode="inline"
          />
          <div className="flex justify-end">
            <button
              type="button"
              onClick={close}
              className="rounded-full border border-slate-700 px-4 py-2 text-xs font-semibold text-slate-200 hover:border-sky-500"
            >
              Done
            </button>
          </div>
        </div>
      ),
    });
  };

  const addEntry = () => {
    const placeholder = defaultForKind(node.kind.item);
    const next = [...entries, placeholder];
    onChange(node.pointer, next);
    editEntry(next.length - 1, placeholder);
  };

  const removeEntry = (index: number) => {
    const next = entries.filter((_, idx) => idx !== index);
    onChange(node.pointer, next);
  };

  return (
    <div className="space-y-2">
      {entries.map((entry, index) => (
        <div
          key={`${node.pointer}-${index}`}
          className="flex items-center justify-between rounded-lg border border-theme bg-panel px-3 py-2 text-xs text-primary"
        >
          <span className="truncate">
            [{index + 1}] {formatValueSummary(entry)}
          </span>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => editEntry(index, entry)}
              className="text-xs text-[var(--app-accent)] hover:text-[var(--app-accent)]"
            >
              Edit
            </button>
            <button
              type="button"
              onClick={() => removeEntry(index)}
              className="text-xs text-rose-300 hover:text-rose-200"
            >
              Remove
            </button>
          </div>
        </div>
      ))}
      <button
        type="button"
        onClick={addEntry}
        className="text-xs font-semibold text-primary hover:text-[var(--app-accent)]"
      >
        + Add entry
      </button>
    </div>
  );
}

function renderCompositeControl(
  node: CompositeNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
  overlay: ReturnType<typeof useOverlay>,
) {
  const { variants, allow_multiple, mode } = node.kind;
  if (!variants.length) {
    return <p className="text-xs text-slate-500">No variants configured.</p>;
  }

  if (allow_multiple) {
    const entries = Array.isArray(value) ? (value as JsonValue[]) : [];
    return (
      <div className="space-y-2">
        {entries.map((entry, index) => {
          const activeVariant = determineVariant(entry, variants);
          return (
            <div
              key={`${node.pointer}-variant-${index}`}
              className="rounded-lg border border-theme bg-panel px-3 py-2 text-xs text-primary"
            >
              <div className="flex items-center justify-between gap-2">
                <span>{activeVariant?.title ?? `Variant ${index + 1}`}</span>
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() => {
                      const entryNode: UiNode = {
                        pointer: `${node.pointer}/${index}`,
                        title: activeVariant?.title ?? `Variant ${index + 1}`,
                        description: activeVariant?.description,
                        required: false,
                        default_value: node.default_value,
                        kind: activeVariant?.node ?? variants[0].node,
                      };
                      overlay.open({
                        title: `${node.title ?? node.pointer} · Variant entry`,
                        content: (close) => (
                          <div className="space-y-4">
                            <NodeRenderer
                              node={entryNode}
                              value={entry}
                              errors={errors}
                              onChange={(_pointer, newValue) => {
                                const next = [...entries];
                                next[index] = newValue;
                                onChange(node.pointer, next);
                              }}
                              renderMode="inline"
                            />
                            <div className="flex justify-end">
                              <button
                                type="button"
                                onClick={close}
                                className="rounded-full border border-slate-700 px-4 py-2 text-xs font-semibold text-slate-200 hover:border-sky-500"
                              >
                                Done
                              </button>
                            </div>
                          </div>
                        ),
                      });
                    }}
                    className="text-xs text-[var(--app-accent)] hover:text-[var(--app-accent)]"
                  >
                    Edit
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      const next = entries.filter((_, idx) => idx !== index);
                      onChange(node.pointer, next);
                    }}
                    className="text-xs text-rose-300 hover:text-rose-200"
                  >
                    Remove
                  </button>
                </div>
              </div>
            </div>
          );
        })}
        <button
          type="button"
          onClick={() =>
            onChange(node.pointer, [...entries, variantDefault(variants[0])])}
          className="text-xs font-semibold text-slate-300 hover:text-sky-200"
        >
          + Add variant entry
        </button>
      </div>
    );
  }

  const activeVariant = determineVariant(value, variants) ?? variants[0];
  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-2 text-xs">
        {variants.map((variant) => (
          <label
            key={variant.id}
            className="inline-flex cursor-pointer items-center gap-2 text-slate-300"
          >
            <input
              type="radio"
              name={node.pointer}
              checked={variant.id === activeVariant.id}
              onChange={() => onChange(node.pointer, variantDefault(variant))}
              className="accent-sky-400"
            />
            <span>{variant.title ?? variant.id}</span>
          </label>
        ))}
      </div>
      <button
        type="button"
        onClick={() =>
          overlay.open({
            title: `${node.title ?? node.pointer} · ${
              activeVariant.title ?? "Variant"
            }`,
            content: (close) => (
              <div className="space-y-4">
                <NodeRenderer
                  node={{
                    pointer: node.pointer,
                    title: activeVariant.title ?? node.title,
                    description: activeVariant.description ?? node.description,
                    required: node.required,
                    default_value: node.default_value,
                    kind: activeVariant.node,
                  }}
                  value={value}
                  errors={errors}
                  onChange={onChange}
                  renderMode="inline"
                />
                <div className="flex justify-end">
                  <button
                    type="button"
                    onClick={close}
                    className="rounded-full border border-slate-700 px-4 py-2 text-xs font-semibold text-slate-200 hover:border-sky-500"
                  >
                    Done
                  </button>
                </div>
              </div>
            ),
          })}
        className="text-xs font-semibold text-slate-300 hover:text-sky-200"
      >
        Edit variant ({mode === "one_of" ? "single" : "any"})
      </button>
    </div>
  );
}

function determineVariant(value: JsonValue | undefined, variants: UiVariant[]) {
  return variants.find((variant) => variantMatches(value, variant.schema)) ??
    variants[0];
}

function extractChildValue(
  container: JsonValue | undefined,
  pointer: string,
): JsonValue | undefined {
  if (container === null || container === undefined) {
    return undefined;
  }
  const token = pointerSegment(pointer);
  if (!token) {
    return undefined;
  }
  if (Array.isArray(container)) {
    const index = Number(token);
    return Number.isNaN(index) ? undefined : container[index];
  }
  if (typeof container === "object") {
    return (container as Record<string, JsonValue>)[token];
  }
  return undefined;
}

function pointerSegment(pointer: string): string | undefined {
  if (!pointer || pointer === "/") {
    return undefined;
  }
  const segments = pointer.split("/").filter(Boolean);
  const raw = segments[segments.length - 1];
  return raw?.replace(/~1/g, "/").replace(/~0/g, "~");
}

function formatValueSummary(value: JsonValue | undefined): string {
  if (value === null || value === undefined) return "empty";
  if (typeof value === "string") return value || '""';
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (Array.isArray(value)) return `[items: ${value.length}]`;
  if (typeof value === "object") {
    const keys = Object.keys(value as Record<string, JsonValue>);
    return keys.length ? `{ ${keys.slice(0, 3).join(", ")} }` : "{}";
  }
  return "value";
}
