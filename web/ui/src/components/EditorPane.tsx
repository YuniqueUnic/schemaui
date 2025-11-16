import { Fragment, memo, useEffect, useMemo, useState } from "react";
import { clsx } from "clsx";
import type {
  JsonValue,
  WebCompositeVariant,
  WebField,
  WebFieldKind,
  WebSection,
} from "../types";
import {
  type UiArrayNode,
  type UiCompositeNode,
  type UiNode,
  type UiSection,
  toUiNode,
} from "../ui-ast";
import {
  deepClone,
  getPointerValue,
  setPointerValue,
} from "../utils/jsonPointer";
import { useTheme } from "../theme";
import { ChevronRight } from "lucide-react";

interface EditorPaneProps {
  section?: UiSection;
  data: JsonValue;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  breadcrumbs: string[];
  loading?: boolean;
}

export const EditorPane = memo(function EditorPane({
  section,
  data,
  errors,
  onChange,
  breadcrumbs,
  loading = false,
}: EditorPaneProps) {
  if (loading) {
    return (
      <div className="flex h-full flex-col gap-4 overflow-auto px-8 py-6">
        <div className="h-6 w-56 animate-pulse rounded-full bg-slate-200 dark:bg-slate-800/50" />
        <div className="space-y-4">
          {Array.from({ length: 5 }).map((_, index) => (
            <div
              key={`skeleton-${index}`}
              className="h-28 animate-pulse rounded-2xl bg-slate-100 dark:bg-slate-800/40"
            />
          ))}
        </div>
      </div>
    );
  }

  if (!section || (!section.children?.length && !section.sections?.length)) {
    return (
      <div className="flex h-full flex-col items-center justify-center text-center text-sm text-slate-600 dark:text-slate-400">
        <p>No fields in this section.</p>
        <p className="text-xs text-slate-500 dark:text-slate-500">
          Select another node from the tree.
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-4 overflow-auto px-8 py-6">
      {breadcrumbs.length
        ? (
          <nav className="flex flex-wrap items-center gap-2 text-xs text-slate-500 dark:text-slate-500">
            {breadcrumbs.map((crumb, index) => (
              <Fragment key={`${crumb}-${index}`}>
                <span className="rounded-full bg-slate-100 px-3 py-1 text-slate-600 dark:bg-slate-900/70 dark:text-slate-300">
                  {crumb}
                </span>
                {index < breadcrumbs.length - 1
                  ? (
                    <ChevronRight className="h-3.5 w-3.5 text-slate-400 dark:text-slate-600" />
                  )
                  : null}
              </Fragment>
            ))}
          </nav>
        )
        : null}
      <article className="rounded-2xl border border-slate-200 bg-white/90 p-6 shadow-lg dark:border-slate-800/70 dark:bg-slate-900/40">
        <header>
          <p className="text-xs uppercase tracking-[0.25em] text-slate-500 dark:text-slate-400">
            Section
          </p>
          <h2 className="text-2xl font-semibold">{section.title}</h2>
          {section.description
            ? (
              <p className="mt-2 text-sm text-slate-600 dark:text-slate-400">
                {section.description}
              </p>
            )
            : null}
        </header>
        <div className="mt-6 space-y-5">
          {section.children?.map((node) => (
            <NodeRenderer
              key={node.pointer || node.id}
              node={node}
              value={getPointerValue(data, node.pointer)}
              error={errors.get(node.pointer)}
              errors={errors}
              onChange={onChange}
              data={data}
            />
          ))}
        </div>
      </article>
    </div>
  );
});

interface NodeRendererProps {
  node: UiNode;
  value: JsonValue | undefined;
  error?: string;
  errors: Map<string, string>;
  data: JsonValue;
  onChange: (pointer: string, value: JsonValue) => void;
}

function NodeRenderer({
  node,
  value,
  error,
  onChange,
  data,
  errors,
}: NodeRendererProps) {
  const pointer = node.pointer;
  const id = pointer || node.id;
  const typeLabel = describeNodeKind(node);
  const body = useMemo(() => {
    if (!pointer) {
      return null;
    }
    if (node.kind === "field") {
      switch (node.fieldType) {
        case "string":
          return (
            <input
              id={id}
              type="text"
              className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 outline-none ring-brand-400/50 focus:border-brand-400 focus:ring-2 dark:border-slate-700/70 dark:bg-slate-900/40 dark:text-slate-100"
              value={(value as string) ?? ""}
              onChange={(event) => onChange(pointer, event.target.value)}
              spellCheck={false}
            />
          );
        case "integer":
        case "number": {
          const parsedValue = typeof value === "number"
            ? value
            : typeof value === "string"
            ? Number(value)
            : "";
          return (
            <input
              id={id}
              type="number"
              className="rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 outline-none ring-brand-400/50 focus:border-brand-400 focus:ring-2 dark:border-slate-700/70 dark:bg-slate-900/40 dark:text-slate-100"
              value={Number.isNaN(parsedValue) ? "" : parsedValue}
              onChange={(event) => {
                const next = event.target.value;
                onChange(pointer, next === "" ? null : Number(next));
              }}
            />
          );
        }
        case "boolean":
          return (
            <label className="inline-flex items-center gap-3">
              <input
                id={id}
                type="checkbox"
                className="h-5 w-5 accent-brand-400"
                checked={Boolean(value)}
                onChange={(event) => onChange(pointer, event.target.checked)}
              />
              <span className="text-sm text-slate-700 dark:text-slate-200">
                Enabled
              </span>
            </label>
          );
        case "enum":
          return (
            <select
              id={id}
              className="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 outline-none ring-brand-400/50 focus:border-brand-400 focus:ring-2 dark:border-slate-700/70 dark:bg-slate-900/40 dark:text-slate-100"
              value={(value as string) ?? node.options?.[0] ?? ""}
              onChange={(event) => onChange(pointer, event.target.value)}
            >
              {(node.options || []).map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </select>
          );
        default:
          return (
            <textarea
              id={id}
              rows={6}
              className="rounded-2xl border border-slate-200 bg-white px-3 py-2 font-mono text-sm text-slate-900 outline-none ring-brand-400/50 focus:border-brand-400 focus:ring-2 dark:border-slate-700/70 dark:bg-slate-900/40 dark:text-slate-100"
              value={value ? JSON.stringify(value, null, 2) : ""}
              onChange={(event) => {
                const text = event.target.value;
                if (!text.trim()) {
                  onChange(pointer, null);
                  return;
                }
                try {
                  const parsed = JSON.parse(text) as JsonValue;
                  onChange(pointer, parsed);
                } catch (parseError) {
                  console.error(parseError);
                }
              }}
              spellCheck={false}
            />
          );
      }
    }
    if (node.kind === "array") {
      if (isCompositeKind(node.items)) {
        return (
          <CompositeArrayEditor
            node={node}
            value={Array.isArray(value) ? value : []}
            data={data}
            errors={errors}
            onChange={onChange}
          />
        );
      }
      return (
        <PrimitiveArrayEditor
          pointer={pointer}
          itemKind={node.items?.type}
          value={Array.isArray(value) ? value : []}
          onChange={onChange}
        />
      );
    }
    if (node.kind === "composite") {
      return (
        <CompositeFieldEditor
          node={node}
          value={value}
          data={data}
          errors={errors}
          onChange={onChange}
        />
      );
    }
    return (
      <textarea
        id={id}
        rows={6}
        className="rounded-2xl border border-slate-200 bg-white px-3 py-2 font-mono text-sm text-slate-900 outline-none ring-brand-400/50 focus:border-brand-400 focus:ring-2 dark:border-slate-700/70 dark:bg-slate-900/40 dark:text-slate-100"
        value={value ? JSON.stringify(value, null, 2) : ""}
        onChange={(event) => {
          const text = event.target.value;
          if (!text.trim()) {
            onChange(pointer, null);
            return;
          }
          try {
            const parsed = JSON.parse(text) as JsonValue;
            onChange(pointer, parsed);
          } catch (parseError) {
            console.error(parseError);
          }
        }}
        spellCheck={false}
      />
    );
  }, [data, errors, id, node, onChange, pointer, value]);

  return (
    <section className="rounded-3xl border border-slate-200 bg-white/95 p-5 shadow-sm transition hover:border-brand-200/70 dark:border-slate-800/70 dark:bg-slate-900/40">
      <header className="flex flex-wrap items-center gap-3">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-slate-400 dark:text-slate-500">
            Field
          </p>
          <h3 className="text-lg font-semibold">{node.label}</h3>
          {pointer
            ? (
              <p className="font-mono text-[10px] text-slate-500 dark:text-slate-400">
                {pointer}
              </p>
            )
            : null}
        </div>
        <span className="ml-auto rounded-full border border-slate-200 px-3 py-1 text-xs uppercase tracking-[0.2em] text-slate-500 dark:border-slate-700 dark:text-slate-400">
          {typeLabel}
        </span>
        {node.required
          ? (
            <span className="rounded-full bg-rose-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] text-rose-500 dark:bg-rose-500/20 dark:text-rose-200">
              Required
            </span>
          )
          : null}
      </header>
      {node.description
        ? (
          <p className="mt-2 text-sm text-slate-600 dark:text-slate-400">
            {node.description}
          </p>
        )
        : null}
      <div className="mt-4">{body}</div>
      {error
        ? (
          <p className="mt-2 text-xs text-rose-600 dark:text-rose-300">
            {error}
          </p>
        )
        : null}
    </section>
  );
}

interface PrimitiveArrayEditorProps {
  pointer: string;
  itemKind?: string;
  value: JsonValue[];
  onChange: (pointer: string, value: JsonValue) => void;
}

function PrimitiveArrayEditor({
  pointer,
  itemKind = "string",
  value,
  onChange,
}: PrimitiveArrayEditorProps) {
  const handleChange = (index: number, next: JsonValue) => {
    const copy = [...value];
    copy[index] = next;
    onChange(pointer, copy);
  };

  const addItem = () => {
    const placeholder = defaultArrayValue(itemKind);
    onChange(pointer, [...value, placeholder]);
  };

  const removeItem = (index: number) => {
    const copy = value.filter((_, idx) => idx !== index);
    onChange(pointer, copy);
  };

  return (
    <div className="space-y-3">
      {value.map((entry, index) => (
        <Fragment key={`${pointer}-${index}`}>
          <div className="flex items-center gap-2">
            <span className="text-xs text-slate-500">[{index}]</span>
            <input
              type={itemKind === "boolean"
                ? "text"
                : itemKind === "number" || itemKind === "integer"
                ? "number"
                : "text"}
              className="flex-1 rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 outline-none focus:border-brand-400 dark:border-slate-700/70 dark:bg-slate-900/40 dark:text-slate-100"
              value={typeof entry === "string" || typeof entry === "number"
                ? String(entry)
                : entry === null
                ? ""
                : JSON.stringify(entry)}
              onChange={(event) => {
                if (itemKind === "number" || itemKind === "integer") {
                  const num = Number(event.target.value);
                  handleChange(index, Number.isNaN(num) ? null : num);
                } else if (itemKind === "boolean") {
                  handleChange(index, event.target.value === "true");
                } else {
                  handleChange(index, event.target.value);
                }
              }}
            />
            <button
              type="button"
              onClick={() => removeItem(index)}
              className="rounded-full border border-slate-200 px-3 py-1 text-xs text-slate-600 transition hover:border-rose-400 hover:text-rose-500 dark:border-slate-700 dark:text-slate-300 dark:hover:text-rose-300"
            >
              Remove
            </button>
          </div>
        </Fragment>
      ))}
      <button
        type="button"
        onClick={addItem}
        className="rounded-full border border-dashed border-slate-600 px-4 py-2 text-xs font-medium text-slate-200 transition hover:border-brand-400 hover:text-brand-300"
      >
        Add entry
      </button>
    </div>
  );
}

interface CompositeArrayEditorProps {
  node: UiArrayNode;
  value: JsonValue[];
  data: JsonValue;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
}

function CompositeArrayEditor({
  node,
  value,
  data,
  errors,
  onChange,
}: CompositeArrayEditorProps) {
  const pointer = node.pointer;
  const itemKind = node.items as Extract<WebFieldKind, { type: "composite" }>;
  const entries = Array.isArray(value) ? value : [];
  const variants = itemKind?.variants || [];
  const [nextVariantId, setNextVariantId] = useState<string | undefined>(
    variants[0]?.id,
  );
  const [overlayIndex, setOverlayIndex] = useState<number | null>(null);

  const handleAddEntry = () => {
    if (!nextVariantId) {
      return;
    }
    const variant = variants.find((entry) => entry.id === nextVariantId);
    if (!variant) {
      return;
    }
    const draft = buildVariantDefault(variant, pointer);
    onChange(pointer, [...entries, draft]);
  };

  const handleRemove = (index: number) => {
    const next = entries.filter((_, idx) => idx !== index);
    onChange(pointer, next);
  };

  const handleVariantRebuild = (index: number, variantId: string) => {
    if (!variantId) {
      return;
    }
    const variant = variants.find((entry) => entry.id === variantId);
    if (!variant) {
      return;
    }
    const next = [...entries];
    next[index] = buildVariantDefault(variant, pointer);
    onChange(pointer, next);
  };

  const openOverlay = (index: number) => {
    setOverlayIndex(index);
  };

  const closeOverlay = () => setOverlayIndex(null);

  const overlayPointer =
    overlayIndex !== null ? appendPointerSegment(pointer, overlayIndex.toString()) : null;

  return (
    <div className="space-y-4">
      {entries.map((entry, index) => {
        const entryPointer = appendPointerSegment(pointer, index.toString());
        const variantId = detectVariantId(entry, variants);
        const variant = variantId
          ? variants.find((candidate) => candidate.id === variantId)
          : undefined;
        return (
          <article
            key={`${entryPointer}`}
            className="rounded-2xl border border-slate-200 bg-white/80 p-4 dark:border-slate-800/60 dark:bg-slate-950/30"
          >
            <header className="flex flex-wrap items-center gap-3">
              <div>
                <p className="text-sm font-semibold">Entry #{index + 1}</p>
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  {variant ? variant.title : "Unknown variant"}
                </p>
              </div>
              <button
                type="button"
                onClick={() => openOverlay(index)}
                className="ml-auto rounded-full border border-slate-200 px-3 py-1 text-xs font-medium text-slate-600 transition hover:border-brand-400 hover:text-brand-500 dark:border-slate-700 dark:text-slate-200"
              >
                Edit overlay
              </button>
              <button
                type="button"
                onClick={() => handleRemove(index)}
                className="rounded-full border border-slate-200 px-3 py-1 text-xs text-slate-500 transition hover:border-rose-400 hover:text-rose-500 dark:border-slate-700 dark:text-slate-300"
              >
                Remove
              </button>
            </header>
            <pre className="mt-3 max-h-32 overflow-auto rounded-xl bg-slate-900/5 p-3 text-xs text-slate-600 dark:bg-slate-900/40 dark:text-slate-200">
              {renderEntrySummary(entry)}
            </pre>
          </article>
        );
      })}
      {variants.length
        ? (
          <div className="flex flex-wrap items-center gap-3 rounded-2xl border border-dashed border-slate-300 p-4 dark:border-slate-700">
            <label className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">
              Variant
              <select
                className="rounded-lg border border-slate-300 px-2 py-1 text-xs text-slate-700 dark:border-slate-700 dark:bg-slate-900/60 dark:text-slate-100"
                value={nextVariantId ?? ""}
                onChange={(event) =>
                  setNextVariantId(event.target.value || undefined)}
              >
                {variants.map((variant) => (
                  <option key={variant.id} value={variant.id}>
                    {variant.title}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              onClick={handleAddEntry}
              className="rounded-full border border-slate-300 px-4 py-1 text-xs font-semibold text-slate-600 transition hover:border-brand-400 hover:text-brand-500 dark:border-slate-700 dark:text-slate-200"
            >
              Add entry
            </button>
          </div>
        )
        : (
          <p className="text-sm text-slate-500 dark:text-slate-400">
            No variants available for this array.
          </p>
        )}
      {overlayIndex !== null && overlayPointer
        ? (
          <CompositeEntryOverlay
            index={overlayIndex}
            basePointer={pointer}
            entryPointer={overlayPointer}
            variants={variants}
            data={data}
            errors={errors}
            value={entries[overlayIndex]}
            onChange={onChange}
            onRemove={handleRemove}
            onVariantChange={handleVariantRebuild}
            onClose={closeOverlay}
          />
        )
        : null}
    </div>
  );
}

function defaultArrayValue(kind?: string): JsonValue {
  switch (kind) {
    case "number":
    case "integer":
      return 0;
    case "boolean":
      return false;
    default:
      return "";
  }
}

interface CompositeFieldEditorProps {
  node: UiCompositeNode;
  value: JsonValue | undefined;
  data: JsonValue;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
}

function CompositeFieldEditor({
  node,
  value,
  data,
  errors,
  onChange,
}: CompositeFieldEditorProps) {
  const variants = node.variants || [];
  const isMulti = node.mode === "any_of";
  const isAllOf = node.mode === "all_of";
  const derivedVariantId = useMemo(() => {
    if (isMulti || isAllOf) {
      return undefined;
    }
    return detectVariantId(value, variants);
  }, [isAllOf, isMulti, value, variants]);
  const [selectedVariant, setSelectedVariant] = useState<string | undefined>(
    () => (!isMulti && !isAllOf ? derivedVariantId || variants[0]?.id : undefined),
  );

  useEffect(() => {
    if (isMulti || isAllOf) {
      return;
    }
    if (derivedVariantId && derivedVariantId !== selectedVariant) {
      setSelectedVariant(derivedVariantId);
    }
  }, [derivedVariantId, isAllOf, isMulti, selectedVariant]);

  const entryMetadata = useMemo(() => {
    const map = new Map<string, { index: number }>();
    if (!isMulti || !Array.isArray(value)) {
      return map;
    }
    value.forEach((entry, index) => {
      const variantId = detectVariantId(entry, variants);
      if (variantId && !map.has(variantId)) {
        map.set(variantId, { index });
      }
    });
    return map;
  }, [isMulti, value, variants]);

  const unmatchedEntries = useMemo(() => {
    if (!isMulti || !Array.isArray(value)) {
      return [];
    }
    return value
      .map((entry, index) => ({
        index,
        value: entry,
        variantId: detectVariantId(entry, variants),
      }))
      .filter((item) => !item.variantId);
  }, [isMulti, value, variants]);

  if (isAllOf) {
    return (
      <div className="space-y-4">
        {variants.map((variant) => (
          <article
            key={variant.id}
            className="rounded-2xl border border-slate-200 bg-white/80 p-4 dark:border-slate-800/60 dark:bg-slate-950/30"
          >
            <header className="flex flex-wrap items-center gap-3">
              <h4 className="text-sm font-semibold">{variant.title}</h4>
            </header>
            {variant.description
              ? (
                <p className="mt-2 text-xs text-slate-500 dark:text-slate-400">
                  {variant.description}
                </p>
              )
              : null}
            <div className="mt-3">
              <VariantSectionList
                sections={variant.sections}
                data={data}
                errors={errors}
                onChange={onChange}
              />
            </div>
          </article>
        ))}
      </div>
    );
  }

  if (!variants.length) {
    return (
      <p className="text-sm text-slate-500 dark:text-slate-400">
        No variants available for this schema node.
      </p>
    );
  }

  const handleSingleVariantSelect = (variantId: string) => {
    if (isMulti || variantId === selectedVariant) {
      return;
    }
    setSelectedVariant(variantId);
    const variant = variants.find((entry) => entry.id === variantId);
    if (!variant) {
      return;
    }
    const defaults = buildVariantDefault(variant, node.pointer);
    onChange(node.pointer, defaults);
  };

  const handleMultiVariantToggle = (variantId: string, next: boolean) => {
    if (!isMulti) {
      return;
    }
    const entries = Array.isArray(value) ? [...value] : [];
    const existing = entryMetadata.get(variantId);
    if (!next) {
      if (existing === undefined) {
        return;
      }
      const prune = entries.filter((_, idx) => idx !== existing.index);
      onChange(node.pointer, prune);
      return;
    }
    if (existing !== undefined) {
      return;
    }
    const variant = variants.find((entry) => entry.id === variantId);
    if (!variant) {
      return;
    }
    const defaults = buildVariantDefault(variant, node.pointer);
    const insertAt = computeInsertIndex(
      variantId,
      variants,
      entryMetadata,
      entries.length,
    );
    entries.splice(insertAt, 0, defaults);
    onChange(node.pointer, entries);
  };

  const handleClear = () => {
    if (isMulti) {
      onChange(node.pointer, []);
      return;
    }
    setSelectedVariant(undefined);
    const fallback = node.required ? {} : null;
    onChange(node.pointer, fallback as JsonValue);
  };

  const activeVariant = !isMulti
    ? variants.find((variant) => variant.id === selectedVariant) ?? variants[0]
    : undefined;

  const activeVariantEntries = isMulti
    ? variants
        .map((variant) => {
          const entry = entryMetadata.get(variant.id);
          return entry
            ? { variant, entryIndex: entry.index }
            : null;
        })
        .filter((entry): entry is {
          variant: WebCompositeVariant;
          entryIndex: number;
        } => Boolean(entry))
    : [];

  return (
    <div className="space-y-4">
      <VariantSelector
        pointer={node.pointer}
        mode={node.mode}
        variants={variants}
        selectedId={!isMulti ? selectedVariant : undefined}
        activeIds={isMulti ? new Set(entryMetadata.keys()) : undefined}
        onSelectVariant={handleSingleVariantSelect}
        onToggleVariant={handleMultiVariantToggle}
        onClear={handleClear}
      />
      {!isMulti && activeVariant?.description
        ? (
          <p className="text-sm text-slate-600 dark:text-slate-400">
            {activeVariant.description}
          </p>
        )
        : null}
      {!isMulti && activeVariant
        ? (
          <VariantSectionList
            sections={activeVariant.sections}
            data={data}
            errors={errors}
            onChange={onChange}
          />
        )
        : null}
      {isMulti && (activeVariantEntries.length > 0 || unmatchedEntries.length > 0)
        ? (
          <div className="space-y-4">
            {activeVariantEntries.map(({ variant, entryIndex }) => {
              const entryPointer = appendPointerSegment(
                node.pointer,
                entryIndex.toString(),
              );
              const remappedSections = remapVariantSections(
                variant.sections,
                node.pointer,
                entryPointer,
              );
              return (
                <article
                  key={`${variant.id}-${entryIndex}`}
                  className="rounded-2xl border border-slate-200 bg-white/80 p-4 dark:border-slate-800/60 dark:bg-slate-950/30"
                >
                  <header className="flex flex-wrap items-center gap-3">
                    <h4 className="text-sm font-semibold">
                      {variant.title}
                      <span className="ml-2 text-xs text-slate-500">
                        Entry #{entryIndex + 1}
                      </span>
                    </h4>
                    <span className="ml-auto" />
                    <button
                      type="button"
                      onClick={() => handleMultiVariantToggle(variant.id, false)}
                      className="rounded-full border border-slate-200 px-3 py-1 text-xs font-medium text-slate-500 hover:border-rose-400 hover:text-rose-500 dark:border-slate-700 dark:text-slate-300"
                    >
                      Remove
                    </button>
                  </header>
                  {variant.description
                    ? (
                      <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                        {variant.description}
                      </p>
                    )
                    : null}
                  <div className="mt-3">
                    <VariantSectionList
                      sections={remappedSections}
                      data={data}
                      errors={errors}
                      onChange={onChange}
                    />
                  </div>
                </article>
              );
            })}
            {unmatchedEntries.length
              ? (
                <div className="space-y-3">
                  <p className="text-xs text-amber-600 dark:text-amber-300">
                    The entries below could not be matched to a known variant.
                    You can edit the raw JSON or remove the entry.
                  </p>
                  {unmatchedEntries.map((entry) => {
                    const entryPointer = appendPointerSegment(
                      node.pointer,
                      entry.index.toString(),
                    );
                    return (
                      <UnknownVariantEntry
                        key={`unknown-${entry.index}`}
                        index={entry.index}
                        pointer={entryPointer}
                        value={entry.value}
                        onChange={onChange}
                        onRemove={() => {
                          if (!Array.isArray(value)) {
                            return;
                          }
                          const next = value.filter((_, idx) => idx !== entry.index);
                          onChange(node.pointer, next);
                        }}
                      />
                    );
                  })}
                </div>
              )
              : null}
          </div>
        )
        : null}
      {isMulti && activeVariantEntries.length === 0 && unmatchedEntries.length === 0
        ? (
          <p className="text-sm text-slate-500 dark:text-slate-400">
            Select one or more variants to edit.
          </p>
        )
        : null}
    </div>
  );
}

interface VariantSelectorProps {
  pointer: string;
  mode: "one_of" | "any_of" | "all_of";
  variants: WebCompositeVariant[];
  selectedId?: string;
  activeIds?: Set<string>;
  onSelectVariant: (variantId: string) => void;
  onToggleVariant: (variantId: string, next: boolean) => void;
  onClear: () => void;
}

function VariantSelector({
  pointer,
  mode,
  variants,
  selectedId,
  activeIds,
  onSelectVariant,
  onToggleVariant,
  onClear,
}: VariantSelectorProps) {
  const isMulti = mode === "any_of";
  const groupName = `${pointer}-variants`;
  return (
    <section className="rounded-2xl border border-slate-200 bg-white/80 p-4 dark:border-slate-800/60 dark:bg-slate-950/30">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-slate-500 dark:text-slate-400">
            Variants
          </p>
          <p className="text-sm text-slate-600 dark:text-slate-300">
            {isMulti ? "Select one or more entries" : "Choose exactly one option"}
          </p>
        </div>
        <button
          type="button"
          onClick={onClear}
          className="rounded-full border border-slate-300 px-3 py-1 text-xs font-semibold text-slate-500 transition hover:border-rose-400 hover:text-rose-500 dark:border-slate-700 dark:text-slate-300"
        >
          {isMulti ? "Clear selections" : "Reset"}
        </button>
      </div>
      <div className="mt-4 space-y-2">
        {variants.map((variant) => {
          const checked = isMulti
            ? activeIds?.has(variant.id) ?? false
            : selectedId === variant.id;
          return (
            <label
              key={variant.id}
              className={clsx(
                "flex items-start gap-3 rounded-xl border px-3 py-2 transition",
                checked
                  ? "border-brand-400 bg-brand-400/10 text-slate-900 dark:border-brand-300 dark:text-brand-50"
                  : "border-slate-200 text-slate-700 hover:border-brand-300 dark:border-slate-700 dark:text-slate-200",
              )}
            >
              <input
                type={isMulti ? "checkbox" : "radio"}
                name={groupName}
                value={variant.id}
                checked={checked}
                onChange={(event) => {
                  if (isMulti) {
                    onToggleVariant(variant.id, event.target.checked);
                  } else if (event.target.checked) {
                    onSelectVariant(variant.id);
                  }
                }}
                className="mt-1 h-4 w-4 accent-brand-500"
              />
              <div>
                <p className="text-sm font-medium">
                  {variant.title}
                </p>
                {variant.description
                  ? (
                    <p className="text-xs text-slate-500 dark:text-slate-400">
                      {variant.description}
                    </p>
                  )
                  : null}
              </div>
            </label>
          );
        })}
      </div>
    </section>
  );
}

interface VariantSectionListProps {
  sections: WebSection[];
  data: JsonValue;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
}

function VariantSectionList({
  sections,
  data,
  errors,
  onChange,
}: VariantSectionListProps) {
  if (!sections?.length) {
    return (
      <p className="rounded-xl border border-dashed border-slate-300 px-3 py-2 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-300">
        No fields defined for this variant.
      </p>
    );
  }
  return (
    <div className="space-y-4">
      {sections.map((section) => (
        <div
          key={section.id}
          className="rounded-2xl border border-slate-200 bg-white/70 p-4 dark:border-slate-700/70 dark:bg-slate-900/40"
        >
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-semibold">{section.title}</h4>
            {section.description
              ? (
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  {section.description}
                </p>
              )
              : null}
          </div>
          <div className="mt-3 space-y-4">
            {section.fields?.map((child) => {
              const node = toUiNode(child);
              return (
                <NodeRenderer
                  key={node.pointer}
                  node={node}
                  value={getPointerValue(data, node.pointer)}
                  error={errors.get(node.pointer)}
                  errors={errors}
                  onChange={onChange}
                  data={data}
                />
              );
            })}
          </div>
          {section.sections?.length
            ? (
              <div className="mt-4 space-y-4">
                <VariantSectionList
                  sections={section.sections}
                  data={data}
                  errors={errors}
                  onChange={onChange}
                />
              </div>
            )
            : null}
        </div>
      ))}
    </div>
  );
}

interface CompositeEntryOverlayProps {
  index: number;
  basePointer: string;
  entryPointer: string;
  variants: WebCompositeVariant[];
  value: JsonValue;
  data: JsonValue;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  onRemove: (index: number) => void;
  onVariantChange: (index: number, variantId: string) => void;
  onClose: () => void;
}

function CompositeEntryOverlay({
  index,
  basePointer,
  entryPointer,
  variants,
  value,
  data,
  errors,
  onChange,
  onRemove,
  onVariantChange,
  onClose,
}: CompositeEntryOverlayProps) {
  const variantId = detectVariantId(value, variants);
  const variant = variantId
    ? variants.find((entry) => entry.id === variantId)
    : undefined;
  const sections = variant
    ? remapVariantSections(variant.sections, basePointer, entryPointer)
    : [];
  const { theme } = useTheme();
  const backdropClass = clsx(
    "fixed inset-0 z-50 flex items-center justify-center p-6 transition-colors",
    theme === "dark"
      ? "bg-slate-950/80"
      : "bg-slate-900/35 backdrop-blur-sm"
  );
  const panelClass = clsx(
    "relative w-full max-w-4xl rounded-3xl border p-6 shadow-2xl transition-colors",
    theme === "dark"
      ? "border-slate-700 bg-slate-950/95 text-slate-100"
      : "border-slate-200 bg-white text-slate-900"
  );
  const subheadingClass = clsx(
    "text-xs uppercase tracking-[0.3em]",
    theme === "dark" ? "text-slate-400" : "text-slate-500"
  );
  const removeButtonClass = clsx(
    "ml-auto rounded-full border px-4 py-1 text-sm font-medium transition",
    theme === "dark"
      ? "border-rose-400 text-rose-200 hover:bg-rose-500/10"
      : "border-rose-300 text-rose-600 hover:bg-rose-50"
  );
  const closeButtonClass = clsx(
    "rounded-full border px-4 py-1 text-sm transition",
    theme === "dark"
      ? "border-slate-600 text-slate-200 hover:border-slate-400"
      : "border-slate-300 text-slate-600 hover:border-slate-500 hover:bg-slate-50"
  );
  return (
    <div className={backdropClass}>
      <div className={panelClass}>
        <header className="flex flex-wrap items-start gap-3">
          <div>
            <p className={subheadingClass}>
              Composite Entry
            </p>
            <h3 className="text-2xl font-semibold">Entry #{index + 1}</h3>
          </div>
          <button
            type="button"
            onClick={() => onRemove(index)}
            className={removeButtonClass}
          >
            Remove entry
          </button>
          <button
            type="button"
            onClick={onClose}
            className={closeButtonClass}
          >
            Close
          </button>
        </header>
        <div className="mt-4 space-y-4">
          <VariantSelector
            pointer={entryPointer}
            mode="one_of"
            variants={variants}
            selectedId={variantId}
            activeIds={undefined}
            onSelectVariant={(variantKey) => onVariantChange(index, variantKey)}
            onToggleVariant={() => {}}
            onClear={() => {
              if (variants[0]) {
                onVariantChange(index, variants[0].id);
              }
            }}
          />
          {variant
            ? (
              <VariantSectionList
                sections={sections}
                data={data}
                errors={errors}
                onChange={onChange}
              />
            )
            : (
              <UnknownVariantEntry
                index={index}
                pointer={entryPointer}
                value={value}
                onChange={onChange}
                onRemove={() => onRemove(index)}
              />
            )}
        </div>
      </div>
    </div>
  );
}

function remapVariantSections(
  sections: WebSection[],
  basePointer: string,
  entryPointer: string,
): WebSection[] {
  return sections.map((section) => ({
    ...section,
    fields: section.fields?.map((field) => ({
      ...field,
      pointer: remapPointerForEntry(field.pointer, basePointer, entryPointer),
    })) || [],
    sections: remapVariantSections(section.sections || [], basePointer, entryPointer),
  }));
}

function remapPointerForEntry(
  pointer: string,
  basePointer: string,
  entryPointer: string,
): string {
  if (!pointer) {
    return entryPointer;
  }
  const relative = stripPointerPrefix(pointer, basePointer);
  if (!relative || relative === "/") {
    return entryPointer;
  }
  return joinPointer(entryPointer, relative);
}

function appendPointerSegment(pointer: string, segment: string): string {
  const sanitizedSegment = segment.replace(/^\/+/, "");
  if (!pointer || pointer === "/") {
    return `/${sanitizedSegment}`;
  }
  const trimmed = pointer.endsWith("/") ? pointer.slice(0, -1) : pointer;
  return `${trimmed}/${sanitizedSegment}`;
}

function joinPointer(basePointer: string, relative: string): string {
  if (!relative || relative === "/") {
    return basePointer || "/";
  }
  const normalizedRelative = relative.startsWith("/")
    ? relative
    : `/${relative}`;
  if (!basePointer || basePointer === "/") {
    return normalizedRelative;
  }
  const trimmed = basePointer.endsWith("/")
    ? basePointer.slice(0, -1)
    : basePointer;
  return `${trimmed}${normalizedRelative}`;
}

function computeInsertIndex(
  variantId: string,
  variants: WebCompositeVariant[],
  metadata: Map<string, { index: number }>,
  currentLength: number,
): number {
  const order = variants.findIndex((variant) => variant.id === variantId);
  if (order === -1) {
    return currentLength;
  }
  for (let idx = order + 1; idx < variants.length; idx += 1) {
    const neighbor = metadata.get(variants[idx].id);
    if (neighbor) {
      return neighbor.index;
    }
  }
  return currentLength;
}

function detectVariantId(
  value: JsonValue | undefined,
  variants: WebCompositeVariant[],
): string | undefined {
  for (const variant of variants) {
    if (variantMatches(value, variant.schema)) {
      return variant.id;
    }
  }
  return undefined;
}

function variantMatches(
  value: JsonValue | undefined,
  schema: JsonValue,
): boolean {
  // unwrap scalar-holding envelopes produced by __value pointers
  const unwrapped = isJsonObject(value) && Object.prototype.hasOwnProperty.call(value, "__value")
    ? (value as Record<string, JsonValue>).__value
    : value;

  if (!schema || typeof schema !== "object" || Array.isArray(schema)) {
    return areJsonValuesEqual(unwrapped, schema);
  }

  if (Object.prototype.hasOwnProperty.call(schema, "const")) {
    return areJsonValuesEqual(unwrapped, (schema as any).const);
  }

  if (Array.isArray((schema as any).enum)) {
    return (schema as any).enum.some((candidate: JsonValue) =>
      areJsonValuesEqual(unwrapped, candidate)
    );
  }

  const schemaMap = schema as { [key: string]: JsonValue };
  const schemaType = schemaMap.type;
  const typeMatches = matchesSchemaType(schemaType, unwrapped);
  if (schemaType && !typeMatches) {
    return false;
  }

  const properties = schemaMap.properties as
    | Record<string, JsonValue>
    | undefined;
  const required = Array.isArray(schemaMap.required)
    ? (schemaMap.required as JsonValue[]).map(String)
    : [];

  if (!properties || !isJsonObject(unwrapped)) {
    return schemaType ? typeMatches : false;
  }

  let discriminators = 0;
  const objectValue = unwrapped as Record<string, JsonValue>;

  for (const requiredKey of required) {
    if (!Object.prototype.hasOwnProperty.call(objectValue, requiredKey)) {
      return false;
    }
  }

  for (const [key, spec] of Object.entries(properties)) {
    if (!spec || typeof spec !== "object") {
      continue;
    }
    if (Object.prototype.hasOwnProperty.call(spec, "const")) {
      discriminators += 1;
      if (!areJsonValuesEqual(objectValue[key], (spec as any).const)) {
        return false;
      }
      continue;
    }
    if (Array.isArray((spec as any).enum)) {
      discriminators += 1;
      const actual = objectValue[key];
      const matches = (spec as any).enum.some((candidate: JsonValue) =>
        areJsonValuesEqual(candidate, actual)
      );
      if (!matches) {
        return false;
      }
    }
  }
  if (discriminators > 0) {
    return true;
  }
  return schemaType ? typeMatches : true;
}

function matchesSchemaType(
  schemaType: JsonValue | JsonValue[] | undefined,
  value: JsonValue | undefined,
): boolean {
  if (schemaType === undefined) {
    return true;
  }
  const candidates = Array.isArray(schemaType) ? schemaType : [schemaType];
  return candidates.some((candidate) => {
    if (typeof candidate !== "string") {
      return true;
    }
    switch (candidate) {
      case "object":
        return isJsonObject(value);
      case "array":
        return Array.isArray(value);
      case "string":
        return typeof value === "string";
      case "integer":
        return typeof value === "number" && Number.isInteger(value as number);
      case "number":
        return typeof value === "number" && Number.isFinite(value as number);
      case "boolean":
        return typeof value === "boolean";
      case "null":
        return value === null;
      default:
        return true;
    }
  });
}

function areJsonValuesEqual(left: JsonValue | undefined, right: JsonValue) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function buildVariantDefault(
  variant: WebCompositeVariant,
  basePointer: string,
): JsonValue {
  const fields = collectFieldsFromSections(variant.sections);
  let result: JsonValue = variant.is_object ? {} : null;
  for (const child of fields) {
    const relative = stripPointerPrefix(child.pointer, basePointer);
    const normalized = relative || "/";
    const initial = resolveDefaultValueForKind(child.kind, child.default_value);
    result = setRelativeValue(result, normalized, initial);
  }
  return result;
}

function collectFieldsFromSections(sections: WebSection[]): WebField[] {
  const bucket: WebField[] = [];
  const walk = (list: WebSection[]) => {
    list.forEach((section) => {
      section.fields?.forEach((field) => bucket.push(field));
      if (section.sections?.length) {
        walk(section.sections);
      }
    });
  };
  walk(sections);
  return bucket;
}

function resolveDefaultValueForKind(
  kind: WebFieldKind,
  explicit: JsonValue | undefined,
): JsonValue {
  if (explicit !== undefined && explicit !== null) {
    return deepClone(explicit);
  }
  return inferKindDefault(kind);
}

function inferKindDefault(kind: WebFieldKind): JsonValue {
  switch (kind.type) {
    case "string":
      return "";
    case "integer":
    case "number":
      return 0;
    case "boolean":
      return false;
    case "enum":
      return kind.options[0] ?? "";
    case "array":
      return [];
    case "json":
      return {};
    case "key_value":
      return {};
    case "composite":
      return kind.mode === "any_of" ? [] : {};
    default:
      return {};
  }
}

function renderEntrySummary(entry: JsonValue): string {
  if (entry === null || entry === undefined) {
    return "null";
  }
  const text = typeof entry === "string"
    ? entry
    : JSON.stringify(entry, null, 2) ?? "";
  const lines = text.split("\n");
  if (lines.length <= 6) {
    return text;
  }
  return `${lines.slice(0, 5).join("\n")}\n…`;
}

function isJsonObject(
  candidate: JsonValue | undefined,
): candidate is Record<string, JsonValue> {
  return Boolean(candidate) && typeof candidate === "object" && !Array.isArray(candidate);
}

function isCompositeKind(
  kind: WebFieldKind | undefined,
): kind is Extract<WebFieldKind, { type: "composite" }> {
  return Boolean(kind) && kind?.type === "composite";
}

interface UnknownVariantEntryProps {
  index: number;
  pointer: string;
  value: JsonValue;
  onChange: (pointer: string, value: JsonValue) => void;
  onRemove: () => void;
}

function UnknownVariantEntry({
  index,
  pointer,
  value,
  onChange,
  onRemove,
}: UnknownVariantEntryProps) {
  const [draft, setDraft] = useState(() => JSON.stringify(value, null, 2));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDraft(JSON.stringify(value, null, 2));
    setError(null);
  }, [value]);

  const handleBlur = () => {
    try {
      const parsed = draft.trim() ? JSON.parse(draft) : null;
      onChange(pointer, parsed as JsonValue);
      setError(null);
    } catch {
      setError("Invalid JSON payload");
    }
  };

  return (
    <article className="rounded-2xl border border-amber-300/60 bg-amber-50/60 p-4 dark:border-amber-300/40 dark:bg-amber-950/20">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-semibold text-amber-800 dark:text-amber-200">
          Unmatched entry #{index + 1}
        </h4>
        <button
          type="button"
          onClick={onRemove}
          className="rounded-full border border-amber-400 px-3 py-1 text-xs font-medium text-amber-700 hover:bg-amber-100 dark:border-amber-200 dark:text-amber-100"
        >
          Remove
        </button>
      </div>
      <textarea
        className="mt-3 h-32 w-full rounded-xl border border-amber-200 bg-white/90 p-3 font-mono text-xs text-amber-900 outline-none focus:border-brand-400 focus:ring-1 focus:ring-brand-400 dark:border-amber-200/40 dark:bg-slate-950/70 dark:text-amber-100"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={handleBlur}
        spellCheck={false}
      />
      {error
        ? (
          <p className="mt-2 text-xs text-rose-600 dark:text-rose-300">
            {error}
          </p>
        )
        : null}
    </article>
  );
}

function stripPointerPrefix(pointer: string, base: string): string {
  if (!base || base === "/") {
    return pointer || "/";
  }
  const normalizedBase = base.endsWith("/") ? base.slice(0, -1) : base;
  if (pointer === normalizedBase) {
    return "/";
  }
  if (pointer.startsWith(`${normalizedBase}/`)) {
    const remainder = pointer.slice(normalizedBase.length);
    return remainder.startsWith("/") ? remainder : `/${remainder}`;
  }
  if (pointer.startsWith(normalizedBase)) {
    const remainder = pointer.slice(normalizedBase.length);
    if (!remainder) {
      return "/";
    }
    return remainder.startsWith("/") ? remainder : `/${remainder}`;
  }
  return pointer;
}

function setRelativeValue(
  root: JsonValue,
  pointer: string,
  value: JsonValue,
): JsonValue {
  const normalized = pointer.startsWith("/") ? pointer : `/${pointer}`;
  if (normalized === "/") {
    return deepClone(value ?? null);
  }
  return setPointerValue(root ?? {}, normalized, deepClone(value ?? null));
}

function describeNodeKind(node: UiNode): string {
  switch (node.kind) {
    case "field":
      if (node.fieldType === "enum") {
        return `enum (${node.options?.length ?? 0})`;
      }
      return node.fieldType;
    case "array":
      return `array<${describeValueKind(node.items)}>`;
    case "composite": {
      const modeLabel = node.mode === "one_of"
        ? "oneOf"
        : node.mode === "any_of"
        ? "anyOf"
        : "allOf";
      return `${modeLabel} (${node.variants.length})`;
    }
    case "key_value":
      return `map<key, ${describeValueKind(node.spec.value_kind)}>`;
    default:
      return "field";
  }
}

function describeValueKind(kind?: WebFieldKind): string {
  if (!kind) {
    return "any";
  }
  switch (kind.type) {
    case "array":
      return `array<${describeValueKind(kind.items)}>`;
    case "composite": {
      const label = kind.mode === "one_of" ? "oneOf" : "anyOf";
      return `${label}(${kind.variants.length})`;
    }
    case "enum":
      return `enum(${kind.options.length})`;
    default:
      return kind.type;
  }
}
