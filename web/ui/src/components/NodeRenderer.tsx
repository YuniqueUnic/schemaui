import { useOverlay } from './Overlay';
import { variantMatches } from '../utils/variantMatch';
import type { JsonValue, UiNode, UiNodeKind, UiVariant } from '../types';
import { defaultForKind, variantDefault } from '../ui-ast';
import type { ReactNode } from 'react';

interface NodeRendererProps {
  node: UiNode;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  renderMode?: 'stack' | 'inline';
}

export function NodeRenderer({ node, value, errors, onChange, renderMode = 'stack' }: NodeRendererProps) {
  const overlay = useOverlay();
  const error = errors.get(node.pointer);

  if (node.kind.type === 'object') {
    return null;
  }

  const chromeClass =
    renderMode === 'inline'
      ? 'space-y-3'
      : 'space-y-3 rounded-2xl border border-slate-800/60 bg-slate-900/40 px-4 py-3 shadow-[0_10px_60px_rgba(15,23,42,0.25)] backdrop-blur';

  return (
    <div className={chromeClass}>
      <header className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <p className="text-sm font-medium text-slate-100">
            {node.title ?? node.pointer}
            {node.required ? (
              <span className="ml-2 text-xs uppercase tracking-[0.3em] text-rose-400">*</span>
            ) : null}
          </p>
          {node.description ? <p className="text-xs text-slate-400">{node.description}</p> : null}
        </div>
      </header>
      {renderBody(node, value, errors, onChange, overlay)}
      {error ? <p className="text-xs text-rose-400">{error}</p> : null}
    </div>
  );
}

type FieldNode = UiNode & { kind: Extract<UiNodeKind, { type: 'field' }> };
type ArrayNode = UiNode & { kind: Extract<UiNodeKind, { type: 'array' }> };
type CompositeNode = UiNode & { kind: Extract<UiNodeKind, { type: 'composite' }> };

function renderBody(
  node: UiNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
  overlay: ReturnType<typeof useOverlay>,
): ReactNode {
  switch (node.kind.type) {
    case 'field':
      return renderFieldControl(node as FieldNode, value, onChange);
    case 'array':
      return renderArrayControl(node as ArrayNode, value, errors, onChange, overlay);
    case 'composite':
      return renderCompositeControl(node as CompositeNode, value, errors, onChange, overlay);
    default:
      return null;
  }
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
        className="w-full rounded-xl border border-slate-800 bg-slate-950/70 px-3 py-2 text-sm text-slate-100 focus:border-sky-400 focus:outline-none"
        value={(resolved as string) ?? ''}
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
    case 'integer':
    case 'number':
      return (
        <input
          type="number"
          className="w-full rounded-xl border border-slate-800 bg-slate-950/70 px-3 py-2 text-sm text-slate-100 focus:border-sky-400 focus:outline-none"
          value={typeof resolved === 'number' ? resolved : 0}
          onChange={(event) => onChange(node.pointer, Number(event.target.value))}
        />
      );
    case 'boolean':
      return (
        <label className="inline-flex items-center gap-3 text-sm text-slate-200">
          <input
            type="checkbox"
            checked={Boolean(resolved)}
            onChange={(event) => onChange(node.pointer, event.target.checked)}
            className="h-4 w-4 rounded border-slate-700 bg-slate-900 text-sky-400"
          />
          Toggle
        </label>
      );
    case 'string':
    default:
      return (
        <input
          type="text"
          className="w-full rounded-xl border border-slate-800 bg-slate-950/70 px-3 py-2 text-sm text-slate-100 focus:border-sky-400 focus:outline-none"
          value={(resolved as string) ?? ''}
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
    overlay.open({
      title: `${node.title ?? node.pointer} · Item ${index + 1}`,
      content: (close) => (
        <div className="space-y-4">
          <NodeRenderer
            node={{ ...node, kind: node.kind.item }}
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
    <div className="space-y-3">
      {entries.map((entry, index) => (
        <div
          key={`${node.pointer}-${index}`}
          className="flex items-center justify-between rounded-xl border border-slate-800/70 bg-slate-950/40 px-3 py-2 text-xs text-slate-300"
        >
          <span className="truncate">[{index + 1}] {formatValueSummary(entry)}</span>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => editEntry(index, entry)}
              className="rounded-full border border-slate-700 px-3 py-1 text-[10px] font-semibold text-slate-200 hover:border-sky-400"
            >
              Edit
            </button>
            <button
              type="button"
              onClick={() => removeEntry(index)}
              className="rounded-full border border-rose-500/60 px-3 py-1 text-[10px] font-semibold text-rose-300 hover:border-rose-400"
            >
              Remove
            </button>
          </div>
        </div>
      ))}
      <button
        type="button"
        onClick={addEntry}
        className="rounded-full border border-dashed border-slate-700 px-4 py-2 text-xs font-semibold text-slate-200 hover:border-sky-400"
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
    const addVariantEntry = () => {
      const base = variants[0];
      const placeholder = variantDefault(base);
      onChange(node.pointer, [...entries, placeholder]);
    };

    return (
      <div className="space-y-3">
        {entries.map((entry, index) => {
          const activeVariant = determineVariant(entry, variants);
          return (
            <div
              key={`${node.pointer}-variant-${index}`}
              className="rounded-xl border border-slate-800/70 bg-slate-950/40 px-3 py-2 text-xs text-slate-300"
            >
              <div className="flex items-center justify-between gap-2">
                <span>{activeVariant?.title ?? `Variant ${index + 1}`}</span>
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() =>
                      overlay.open({
                        title: `${node.title ?? node.pointer} · Variant entry`,
                        content: (close) => (
                          <div className="space-y-4">
                            <NodeRenderer
                              node={{ ...node, kind: activeVariant?.node ?? variants[0].node }}
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
                      })
                    }
                    className="rounded-full border border-slate-700 px-3 py-1 text-[10px] font-semibold text-slate-200 hover:border-sky-400"
                  >
                    Edit
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      const next = entries.filter((_, idx) => idx !== index);
                      onChange(node.pointer, next);
                    }}
                    className="rounded-full border border-rose-500/60 px-3 py-1 text-[10px] font-semibold text-rose-300 hover:border-rose-400"
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
          onClick={addVariantEntry}
          className="rounded-full border border-dashed border-slate-700 px-4 py-2 text-xs font-semibold text-slate-200 hover:border-sky-400"
        >
          + Add variant entry
        </button>
      </div>
    );
  }

  const activeVariant = determineVariant(value, variants) ?? variants[0];
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2 text-xs">
        {variants.map((variant) => (
          <label key={variant.id} className="inline-flex cursor-pointer items-center gap-2">
            <input
              type="radio"
              name={node.pointer}
              checked={variant.id === activeVariant.id}
              onChange={() => onChange(node.pointer, variantDefault(variant))}
              className="accent-sky-400"
            />
            <span className="rounded-full bg-white/5 px-3 py-1 text-slate-200">
              {variant.title ?? variant.id}
            </span>
          </label>
        ))}
      </div>
      <button
        type="button"
        onClick={() =>
          overlay.open({
            title: `${node.title ?? node.pointer} · ${activeVariant.title ?? 'Variant'}`,
            content: (close) => (
              <div className="space-y-4">
                <NodeRenderer
                  node={{ ...node, kind: activeVariant.node }}
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
          })
        }
        className="rounded-full border border-slate-700 px-4 py-1.5 text-xs font-semibold text-slate-200 hover:border-sky-400"
      >
        Edit variant ({mode === 'one_of' ? 'single' : 'any'})
      </button>
    </div>
  );
}

function determineVariant(value: JsonValue | undefined, variants: UiVariant[]) {
  return variants.find((variant) => variantMatches(value, variant.schema)) ?? variants[0];
}

function formatValueSummary(value: JsonValue | undefined): string {
  if (value === null || value === undefined) return 'empty';
  if (typeof value === 'string') return value || '""';
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (Array.isArray(value)) return `[items: ${value.length}]`;
  if (typeof value === 'object') {
    const keys = Object.keys(value as Record<string, JsonValue>);
    return keys.length ? `{ ${keys.slice(0, 3).join(', ')} }` : '{}';
  }
  return 'value';
}
