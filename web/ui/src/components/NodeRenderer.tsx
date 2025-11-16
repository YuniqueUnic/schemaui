import { useMemo } from 'react';
import type { JsonValue, UiNode, UiNodeKind, UiVariant } from '../types';
import { defaultForKind, variantDefault } from '../ui-ast';
import { useOverlay } from './Overlay';
import { variantMatches } from '../utils/variantMatch';

type RenderMode = 'card' | 'inline';

interface Props {
  node: UiNode;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  renderMode?: RenderMode;
}

export function NodeRenderer({ node, value, errors, onChange, renderMode = 'card' }: Props) {
  const overlay = useOverlay();
  const errorMessage = errors.get(node.pointer);

  const body = useMemo(() => {
    switch (node.kind.type) {
      case 'field':
        return renderField(node, value, onChange);
      case 'array':
        return renderArray(node, value, errors, overlay, onChange);
      case 'object':
        return renderObject(node, value, errors, onChange);
      case 'composite':
        return renderComposite(node, value, errors, onChange, overlay);
      default:
        return null;
    }
  }, [node, value, errors, onChange, overlay]);

  if (renderMode === 'inline') {
    return <div className="space-y-3">{body}</div>;
  }

  return (
    <section className="space-y-2 rounded-xl border border-slate-200 bg-white/90 p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900/60">
      <header className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold text-slate-900 dark:text-slate-100">
            {node.title ?? node.pointer}
            {node.required ? <span className="ml-2 text-rose-500">*</span> : null}
          </p>
          {node.description ? (
            <p className="text-xs text-slate-600 dark:text-slate-400">{node.description}</p>
          ) : null}
        </div>
        {errorMessage ? (
          <span className="rounded-full bg-rose-100 px-3 py-1 text-xs text-rose-700 dark:bg-rose-900/40 dark:text-rose-200">
            {errorMessage}
          </span>
        ) : null}
      </header>
      <div className="pt-2">{body}</div>
    </section>
  );
}

function renderField(
  node: UiNode,
  value: JsonValue | undefined,
  onChange: (pointer: string, value: JsonValue) => void,
) {
  if (node.kind.type !== 'field') return null;
  const v = value ?? node.default_value ?? defaultForKind(node.kind);
  if (node.kind.enum_options?.length) {
    return (
      <select
        className="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-900"
        value={(v as string) ?? ''}
        onChange={(event) => onChange(node.pointer, event.target.value)}
      >
        {node.kind.enum_options.map((opt) => (
          <option key={opt} value={opt}>
            {opt}
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
          className="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-900"
          value={typeof v === 'number' ? v : 0}
          onChange={(event) => onChange(node.pointer, Number(event.target.value))}
        />
      );
    case 'boolean':
      return (
        <label className="inline-flex items-center gap-2 text-sm text-slate-700 dark:text-slate-200">
          <input
            type="checkbox"
            checked={Boolean(v)}
            onChange={(event) => onChange(node.pointer, event.target.checked)}
          />
          Enabled
        </label>
      );
    case 'string':
    default:
      return (
        <input
          type="text"
          className="w-full rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-900"
          value={(v as string) ?? ''}
          onChange={(event) => onChange(node.pointer, event.target.value)}
        />
      );
  }
}

function renderArray(
  node: UiNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  overlay: ReturnType<typeof useOverlay>,
  onChange: (pointer: string, value: JsonValue) => void,
) {
  if (node.kind.type !== 'array') return null;
  const arrayKind = node.kind as Extract<UiNodeKind, { type: 'array' }>;
  const entries = Array.isArray(value) ? (value as JsonValue[]) : [];

  const openEditor = (index: number) => {
    overlay.open({
      title: `${node.title ?? node.pointer} · Item ${index + 1}`,
      content: (close) => (
        <div className="space-y-4">
          <ChildRenderer
            pointer={`${node.pointer}/${index}`}
            kind={arrayKind.item}
            value={entries[index]}
            errors={errors}
            onChange={onChange}
            renderMode="inline"
          />
          <div className="flex justify-end">
            <button
              type="button"
              onClick={close}
              className="rounded-full border border-slate-300 px-4 py-1 text-xs font-semibold text-slate-600 hover:border-brand-400 hover:text-brand-600 dark:border-slate-700 dark:text-slate-200"
            >
              Done
            </button>
          </div>
        </div>
      ),
    });
  };

  const ensureEntry = () => {
    const placeholder = defaultForKind(arrayKind.item);
    const next = [...entries, placeholder];
    onChange(node.pointer, next);
    return next.length - 1;
  };

  const removeEntry = (index: number) => {
    const next = entries.filter((_, idx) => idx !== index);
    onChange(node.pointer, next);
  };

  return (
    <div className="space-y-3">
      {entries.map((entry, index) => (
        <article
          key={`${node.pointer}-${index}`}
          className="rounded-lg border border-slate-200 bg-slate-50/70 p-3 dark:border-slate-800 dark:bg-slate-950/40"
        >
          <header className="mb-2 flex flex-wrap items-center justify-between gap-2 text-xs text-slate-500 dark:text-slate-400">
            <span>Item #{index + 1}</span>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => openEditor(index)}
                className="rounded-full border border-slate-300 px-3 py-1 text-[11px] font-semibold text-slate-600 hover:border-brand-400 hover:text-brand-600 dark:border-slate-700 dark:text-slate-200"
              >
                Edit
              </button>
              <button
                type="button"
                onClick={() => removeEntry(index)}
                className="rounded-full border border-rose-300 px-3 py-1 text-[11px] font-semibold text-rose-600 hover:border-rose-500 dark:border-rose-500/60 dark:text-rose-200"
              >
                Remove
              </button>
            </div>
          </header>
          <p className="text-xs text-slate-600 dark:text-slate-300">{formatValueSummary(entry)}</p>
        </article>
      ))}
      <button
        type="button"
        onClick={() => openEditor(ensureEntry())}
        className="rounded-full border border-dashed border-slate-400 px-4 py-2 text-xs font-semibold text-slate-700 hover:border-brand-500 hover:text-brand-600 dark:border-slate-700 dark:text-slate-200"
      >
        + Add entry
      </button>
    </div>
  );
}

function renderObject(
  node: UiNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
) {
  if (node.kind.type !== 'object') return null;
  const children = node.kind.children ?? [];
  const objectValue =
    value && typeof value === 'object' && !Array.isArray(value)
      ? (value as Record<string, JsonValue>)
      : {};
  return (
    <div className="space-y-4">
      {children.map((child) => {
        const lastSegment = pointerTail(child.pointer);
        const childValue = lastSegment ? objectValue[lastSegment] : undefined;
        return (
          <NodeRenderer
            key={child.pointer}
            node={child}
            value={childValue}
            errors={errors}
            onChange={onChange}
          />
        );
      })}
    </div>
  );
}

function renderComposite(
  node: UiNode,
  value: JsonValue | undefined,
  errors: Map<string, string>,
  onChange: (pointer: string, value: JsonValue) => void,
  overlay: ReturnType<typeof useOverlay>,
) {
  if (node.kind.type !== 'composite') return null;
  const { variants, allow_multiple, mode } = node.kind;
  if (!variants.length) {
    return <p className="text-xs text-slate-500">No variants defined.</p>;
  }

  if (allow_multiple) {
    const entries = Array.isArray(value) ? (value as JsonValue[]) : [];
    const editEntry = (index: number, entryValue?: JsonValue) => {
      const variantForEntry =
        determineVariant(entryValue ?? entries[index], variants) ?? variants[0];
      overlay.open({
        title: `${node.title ?? node.pointer} · Variant entry`,
        content: (close) => (
          <div className="space-y-4">
            <ChildRenderer
              pointer={`${node.pointer}/${index}`}
              kind={variantForEntry.node}
              value={entryValue ?? entries[index]}
              errors={errors}
              onChange={onChange}
              renderMode="inline"
            />
            <div className="flex justify-end">
              <button
                type="button"
                onClick={close}
                className="rounded-full border border-slate-300 px-4 py-1 text-xs font-semibold text-slate-600 hover:border-brand-400 hover:text-brand-600 dark:border-slate-700 dark:text-slate-200"
              >
                Done
              </button>
            </div>
          </div>
        ),
      });
    };
    const removeEntry = (index: number) => {
      const next = entries.filter((_, idx) => idx !== index);
      onChange(node.pointer, next);
    };
    return (
      <div className="space-y-3">
        {entries.map((entry, index) => {
          const activeVariant = determineVariant(entry, variants);
          return (
            <article
              key={`${node.pointer}-variant-${index}`}
              className="rounded-lg border border-slate-200 bg-slate-50/70 p-3 dark:border-slate-800 dark:bg-slate-950/40"
            >
              <header className="mb-2 flex flex-wrap items-center justify-between gap-2 text-xs text-slate-500 dark:text-slate-300">
                <span>{activeVariant?.title ?? `Variant ${index + 1}`}</span>
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() => editEntry(index, entry)}
                    className="rounded-full border border-slate-300 px-3 py-1 text-[11px] font-semibold text-slate-600 hover:border-brand-400 hover:text-brand-600 dark:border-slate-700 dark:text-slate-200"
                  >
                    Edit
                  </button>
                  <button
                    type="button"
                    onClick={() => removeEntry(index)}
                    className="rounded-full border border-rose-300 px-3 py-1 text-[11px] font-semibold text-rose-600 hover:border-rose-500 dark:border-rose-500/60 dark:text-rose-200"
                  >
                    Remove
                  </button>
                </div>
              </header>
              <p className="text-xs text-slate-600 dark:text-slate-300">{formatValueSummary(entry)}</p>
            </article>
          );
        })}
        <button
          type="button"
          onClick={() => {
            const base = variants[0];
            const placeholder = variantDefault(base);
            const next = [...entries, placeholder];
            const nextIndex = next.length - 1;
            onChange(node.pointer, next);
            editEntry(nextIndex, placeholder);
          }}
          className="rounded-full border border-dashed border-slate-400 px-4 py-2 text-xs font-semibold text-slate-700 hover:border-brand-500 hover:text-brand-600 dark:border-slate-700 dark:text-slate-200"
        >
          + Add variant entry
        </button>
      </div>
    );
  }

  const activeVariant = determineVariant(value, variants);

  const selectVariant = (variantId: string) => {
    const target = variants.find((entry) => entry.id === variantId);
    if (!target) {
      return;
    }
    onChange(node.pointer, variantDefault(target));
  };

  const openOverlay = () => {
    overlay.open({
      title: `${node.title ?? node.pointer} · ${activeVariant?.title ?? 'Variant'}`,
      content: (close) => (
        <div className="space-y-4">
          <ChildRenderer
            pointer={node.pointer}
            kind={activeVariant?.node ?? variants[0].node}
            value={value}
            errors={errors}
            onChange={onChange}
            renderMode="inline"
          />
          <div className="flex justify-end">
            <button
              type="button"
              onClick={close}
              className="rounded-full border border-slate-300 px-4 py-1 text-xs font-semibold text-slate-600 hover:border-brand-400 hover:text-brand-600 dark:border-slate-700 dark:text-slate-200"
            >
              Done
            </button>
          </div>
        </div>
      ),
    });
  };

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-3 text-sm">
        {variants.map((variant) => (
          <label key={variant.id} className="inline-flex cursor-pointer items-center gap-2">
            <input
              type="radio"
              name={node.pointer}
              checked={variant.id === activeVariant?.id}
              onChange={() => selectVariant(variant.id)}
            />
            <span className="rounded-full bg-slate-100 px-3 py-1 text-xs font-semibold text-slate-700 dark:bg-slate-900/50 dark:text-slate-200">
              {variant.title ?? variant.id}
            </span>
          </label>
        ))}
      </div>
      <article className="rounded-lg border border-slate-200 bg-slate-50/70 p-3 dark:border-slate-800 dark:bg-slate-950/40">
        <p className="text-xs text-slate-600 dark:text-slate-300">{formatValueSummary(value)}</p>
        <button
          type="button"
          onClick={openOverlay}
          className="mt-3 rounded-full border border-slate-300 px-4 py-1 text-xs font-semibold text-slate-600 hover:border-brand-400 hover:text-brand-600 dark:border-slate-700 dark:text-slate-200"
        >
          Edit details
        </button>
        <p className="mt-1 text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
          Mode: {mode === 'one_of' ? 'select exactly one' : 'select one (multiple allowed)'}
        </p>
      </article>
    </div>
  );
}

function determineVariant(value: JsonValue | undefined, variants: UiVariant[]) {
  return variants.find((variant) => variantMatches(value, variant.schema)) ?? variants[0];
}

function formatValueSummary(value: JsonValue | undefined): string {
  if (value === null || value === undefined) {
    return 'No value';
  }
  if (typeof value === 'string') {
    return value || '""';
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }
  if (Array.isArray(value)) {
    return `[items: ${value.length}]`;
  }
  if (typeof value === 'object') {
    const keys = Object.keys(value as Record<string, JsonValue>);
    return keys.length ? `{ ${keys.slice(0, 3).join(', ')} }` : '{}';
  }
  return 'Value';
}

function pointerTail(pointer: string): string | undefined {
  const parts = pointer.split('/');
  const tail = parts[parts.length - 1];
  if (!tail) return undefined;
  return tail.replace(/~1/g, '/').replace(/~0/g, '~');
}

interface ChildRendererProps {
  pointer: string;
  kind: UiNodeKind;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  renderMode?: RenderMode;
}

function ChildRenderer({
  pointer,
  kind,
  value,
  errors,
  onChange,
  renderMode = 'card',
}: ChildRendererProps) {
  const placeholderNode: UiNode = {
    pointer,
    title: undefined,
    description: undefined,
    required: false,
    default_value: undefined,
    kind,
  };
  return (
    <NodeRenderer
      node={placeholderNode}
      value={value}
      errors={errors}
      onChange={onChange}
      renderMode={renderMode}
    />
  );
}
