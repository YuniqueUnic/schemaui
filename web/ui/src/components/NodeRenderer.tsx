import { useMemo } from 'react';
import type { JsonValue, UiNode, UiNodeKind, UiVariant } from '../types';
import { defaultForKind, variantDefault } from '../ui-ast';

interface Props {
  node: UiNode;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
}

export function NodeRenderer({ node, value, errors, onChange }: Props) {
  const render = () => {
    switch (node.kind.type) {
      case 'field':
        return renderField(node, value, onChange);
      case 'array':
        return renderArray(node, value, errors, onChange);
      case 'object':
        return renderObject(node, value, errors, onChange);
      case 'composite':
        return renderComposite(node, value, errors, onChange);
      default:
        return null;
    }
  };

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
        {errors.get(node.pointer) ? (
          <span className="rounded-full bg-rose-100 px-3 py-1 text-xs text-rose-700 dark:bg-rose-900/40 dark:text-rose-200">
            {errors.get(node.pointer)}
          </span>
        ) : null}
      </header>
      <div className="pt-2">{render()}</div>
    </section>
  );
}

function renderField(
  node: UiNode,
  value: JsonValue | undefined,
  onChange: (pointer: string, value: JsonValue) => void,
) {
  const v = value ?? node.default_value ?? defaultForKind(node.kind);
  if (node.kind.type !== 'field') return null;

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
          value={Number(v) || 0}
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
  onChange: (pointer: string, value: JsonValue) => void,
) {
  if (node.kind.type !== 'array') return null;
  const arrayKind = node.kind as Extract<UiNodeKind, { type: 'array' }>;
  const entries = Array.isArray(value) ? (value as JsonValue[]) : [];
  const addItem = () => {
    const placeholder = defaultForKind(arrayKind.item);
    onChange(node.pointer, [...entries, placeholder]);
  };

  const updateItem = (index: number, nextValue: JsonValue) => {
    const copy = [...entries];
    copy[index] = nextValue;
    onChange(node.pointer, copy);
  };

  const removeItem = (index: number) => {
    const copy = entries.filter((_, idx) => idx !== index);
    onChange(node.pointer, copy);
  };

  return (
    <div className="space-y-3">
      {entries.map((entry, index) => (
        <div
          key={`${node.pointer}-${index}`}
          className="rounded-lg border border-slate-200 bg-slate-50/70 p-3 dark:border-slate-800 dark:bg-slate-950/40"
        >
          <div className="mb-2 flex items-center justify-between text-xs text-slate-500 dark:text-slate-400">
            <span>Item #{index + 1}</span>
            <button
              type="button"
              onClick={() => removeItem(index)}
              className="text-rose-500 hover:underline disabled:opacity-50"
            >
              Remove
            </button>
          </div>
          <ChildRenderer
            pointer={`${node.pointer}/${index}`}
            kind={arrayKind.item}
            value={entry}
            errors={errors}
            onChange={(_, v) => updateItem(index, v)}
          />
        </div>
      ))}
      <button
        type="button"
        onClick={addItem}
        className="rounded-full border border-dashed border-slate-400 px-3 py-2 text-xs font-semibold text-slate-700 hover:border-brand-500 hover:text-brand-600 dark:border-slate-700 dark:text-slate-200"
      >
        + Add item
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
  const objValue = (value && typeof value === 'object' && !Array.isArray(value) ? value : {}) as JsonValue;

  return (
    <div className="space-y-4">
      {children.map((child) => {
        const childValue =
          typeof objValue === 'object' && objValue !== null
            ? (objValue as Record<string, JsonValue>)[child.pointer.split('/').pop() ?? '']
            : undefined;
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
) {
  if (node.kind.type !== 'composite') return null;
  const { variants, allow_multiple, mode } = node.kind;

  if (allow_multiple) {
    const entries = Array.isArray(value) ? (value as JsonValue[]) : [];
    const addEntry = (variant: UiVariant) => {
      onChange(node.pointer, [...entries, variantDefault(variant)]);
    };
    const updateEntry = (index: number, variant: UiVariant, next: JsonValue) => {
      const copy = [...entries];
      copy[index] = next ?? variantDefault(variant);
      onChange(node.pointer, copy);
    };
    const removeEntry = (index: number) => {
      const copy = entries.filter((_, idx) => idx != index);
      onChange(node.pointer, copy);
    };

    return (
      <div className="space-y-3">
        <div className="flex flex-wrap gap-2 text-xs text-slate-600 dark:text-slate-300">
          {variants.map((variant) => (
            <button
              key={variant.id}
              type="button"
              onClick={() => addEntry(variant)}
              className="rounded-full border border-slate-300 px-3 py-1 font-semibold hover:border-brand-500 hover:text-brand-600 dark:border-slate-700"
            >
              + {variant.title ?? variant.id}
            </button>
          ))}
        </div>
        {entries.map((entry, index) => {
          const variant = determineVariant(entry, variants) ?? variants[0];
          return (
            <article
              key={`${node.pointer}-entry-${index}`}
              className="rounded-lg border border-slate-200 bg-slate-50/70 p-3 dark:border-slate-800 dark:bg-slate-950/40"
            >
              <header className="mb-2 flex items-center justify-between text-xs text-slate-500 dark:text-slate-400">
                <span>Variant: {variant?.title ?? variant?.id}</span>
                <button
                  type="button"
                  onClick={() => removeEntry(index)}
                  className="text-rose-500 hover:underline"
                >
                  Remove
                </button>
              </header>
              <VariantSelector
                variants={variants}
                activeId={variant?.id}
                onSelect={(nextId) => {
                  const nextVariant = variants.find((v) => v.id === nextId);
                  if (!nextVariant) return;
                  const defaults = variantDefault(nextVariant);
                  updateEntry(index, nextVariant, defaults);
                }}
              />
              {variant ? (
                <ChildRenderer
                  pointer={`${node.pointer}/${index}`}
                  kind={variant.node}
                  value={entry}
                  errors={errors}
                  onChange={(_, v) => updateEntry(index, variant, v)}
                />
              ) : null}
            </article>
          );
        })}
      </div>
    );
  }

  const activeVariant = useMemo(() => determineVariant(value, variants) ?? variants[0], [value, variants]);

  return (
    <div className="space-y-3">
      <VariantSelector
        variants={variants}
        activeId={activeVariant?.id}
        onSelect={(id) => {
          const target = variants.find((v) => v.id === id);
          if (!target) return;
          onChange(node.pointer, variantDefault(target));
        }}
      />
      {activeVariant ? (
        <ChildRenderer
          pointer={node.pointer}
          kind={activeVariant.node}
          value={value}
          errors={errors}
          onChange={(p, v) => onChange(p, v)}
        />
      ) : null}
      <p className="text-xs text-slate-500 dark:text-slate-400">
        Mode: {mode === 'one_of' ? 'Select exactly one' : 'Select one (can match multiple)'}
      </p>
    </div>
  );
}

function VariantSelector({
  variants,
  activeId,
  onSelect,
}: {
  variants: UiVariant[];
  activeId?: string;
  onSelect: (id: string) => void;
}) {
  return (
    <div className="flex flex-wrap gap-3 text-sm">
      {variants.map((variant) => (
        <label key={variant.id} className="inline-flex cursor-pointer items-center gap-2">
          <input
            type="radio"
            name={`variant-${variant.id}`}
            checked={variant.id === activeId}
            onChange={() => onSelect(variant.id)}
          />
          <span className="rounded-full bg-slate-100 px-3 py-1 text-slate-700 dark:bg-slate-800 dark:text-slate-200">
            {variant.title ?? variant.id}
          </span>
        </label>
      ))}
    </div>
  );
}

function determineVariant(value: JsonValue | undefined, variants: UiVariant[]): UiVariant | undefined {
  return variants.find((variant) => matchesKind(value, variant.node));
}

function matchesKind(value: JsonValue | undefined, kind: UiNodeKind): boolean {
  switch (kind.type) {
    case 'field':
      if (value === undefined || value === null) return false;
      switch (kind.scalar) {
        case 'boolean':
          return typeof value === 'boolean';
        case 'integer':
        case 'number':
          return typeof value === 'number';
        case 'string':
          return typeof value === 'string';
        default:
          return true;
      }
    case 'array':
      return Array.isArray(value);
    case 'object':
      return typeof value === 'object' && value !== null && !Array.isArray(value);
    case 'composite':
      return matchesKind(value, kind.variants[0]?.node ?? { type: 'field', scalar: 'string' } as UiNodeKind);
    default:
      return false;
  }
}

function ChildRenderer({
  pointer,
  kind,
  value,
  errors,
  onChange,
}: {
  pointer: string;
  kind: UiNodeKind;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
}) {
  const placeholderNode: UiNode = {
    pointer,
    title: undefined,
    description: undefined,
    required: false,
    default_value: undefined,
    kind,
  };
  return <NodeRenderer node={placeholderNode} value={value} errors={errors} onChange={onChange} />;
}
