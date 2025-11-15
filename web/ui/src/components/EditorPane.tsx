import { Fragment, memo, useEffect, useMemo, useState } from "react";
import { clsx } from "clsx";
import type {
  JsonValue,
  WebCompositeVariant,
  WebField,
  WebSection,
} from "../types";
import {
  deepClone,
  getPointerValue,
  setPointerValue,
} from "../utils/jsonPointer";
import { ChevronRight } from "lucide-react";

interface EditorPaneProps {
  section?: WebSection;
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

  if (!section || (!section.fields?.length && !section.sections?.length)) {
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
          {section.fields?.map((field) => (
            <FieldControl
              key={field.pointer}
              field={field}
              value={getPointerValue(data, field.pointer)}
              error={errors.get(field.pointer)}
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

interface FieldControlProps {
  field: WebField;
  value: JsonValue | undefined;
  error?: string;
  errors: Map<string, string>;
  data: JsonValue;
  onChange: (pointer: string, value: JsonValue) => void;
}

function FieldControl({
  field,
  value,
  error,
  onChange,
  data,
  errors,
}: FieldControlProps) {
  const id = field.pointer || field.name;
  const label = field.label || field.name;
  const description = field.description;
  const required = field.required;
  const pointer = field.pointer;
  const typeLabel = describeFieldKind(field.kind);

  const body = useMemo(() => {
    if (!pointer) {
      return null;
    }
    switch (field.kind?.type) {
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
            value={(value as string) ?? field.kind.options?.[0] ?? ""}
            onChange={(event) => onChange(pointer, event.target.value)}
          >
            {(field.kind.options || []).map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        );
      case "array":
        return (
          <PrimitiveArrayEditor
            pointer={pointer}
            itemKind={field.kind.items?.type}
            value={Array.isArray(value) ? value : []}
            onChange={onChange}
          />
        );
      case "composite":
        return (
          <CompositeFieldEditor
            field={field}
            value={value}
            data={data}
            errors={errors}
            onChange={onChange}
          />
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
                const parsed = JSON.parse(text);
                onChange(pointer, parsed);
              } catch {
                // keep text but highlight error
              }
            }}
            spellCheck={false}
          />
        );
    }
  }, [data, errors, field.kind, id, onChange, pointer, value]);

  return (
    <section className="rounded-2xl border border-slate-200 bg-white/85 p-4 dark:border-slate-800/60 dark:bg-slate-950/30">
      <div className="flex flex-wrap items-center gap-2">
        <label htmlFor={id} className="text-sm font-medium">
          {label} {required ? <span className="text-rose-500">*</span> : null}
        </label>
        <span className="font-mono text-[10px] text-slate-500">{pointer}</span>
        <span className="ml-auto rounded-full border border-slate-200 px-2 py-0.5 text-[10px] font-semibold uppercase text-slate-600 dark:border-slate-700 dark:text-slate-300">
          {typeLabel}
        </span>
      </div>
      {description
        ? (
          <p className="mt-1 text-xs text-slate-600 dark:text-slate-400">
            {description}
          </p>
        )
        : null}
      <div className="mt-3">{body}</div>
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
  field: WebField;
  value: JsonValue | undefined;
  data: JsonValue;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
}

function CompositeFieldEditor({
  field,
  value,
  data,
  errors,
  onChange,
}: CompositeFieldEditorProps) {
  if (field.kind.type !== "composite") {
    return null;
  }
  type CompositeKind = Extract<WebField["kind"], { type: "composite" }>;
  const compositeKind = field.kind as CompositeKind;
  const variants = compositeKind.variants || [];
  const derivedVariantId = useMemo(
    () => detectVariantId(value, variants),
    [value, variants],
  );
  const [selectedVariant, setSelectedVariant] = useState<string | undefined>(
    derivedVariantId || variants[0]?.id,
  );

  useEffect(() => {
    if (derivedVariantId && derivedVariantId !== selectedVariant) {
      setSelectedVariant(derivedVariantId);
    }
  }, [derivedVariantId, selectedVariant]);

  const activeVariant = variants.find((variant) =>
    variant.id === selectedVariant
  ) ?? variants[0];

  const handleVariantSelect = (variantId: string) => {
    if (variantId === selectedVariant) {
      return;
    }
    setSelectedVariant(variantId);
    const variant = variants.find((entry) => entry.id === variantId);
    if (!variant) {
      return;
    }
    const defaults = buildVariantDefault(variant, field.pointer);
    onChange(field.pointer, defaults);
  };

  const handleClear = () => {
    setSelectedVariant(undefined);
    const fallback = compositeKind.mode === "any_of"
      ? []
      : field.required
      ? {}
      : null;
    onChange(field.pointer, fallback as JsonValue);
  };

  if (!variants.length) {
    return (
      <p className="text-sm text-slate-500 dark:text-slate-400">
        No variants available for this schema node.
      </p>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap gap-3">
        {variants.map((variant) => {
          const active = variant.id === activeVariant?.id;
          return (
            <button
              type="button"
              key={variant.id}
              onClick={() => handleVariantSelect(variant.id)}
              className={clsx(
                "rounded-full border px-4 py-1 text-sm font-medium transition",
                active
                  ? "border-brand-400 bg-brand-400/10 text-brand-600 dark:border-brand-300 dark:text-brand-200"
                  : "border-slate-300 text-slate-700 hover:border-brand-300 hover:text-brand-500 dark:border-slate-700 dark:text-slate-200",
              )}
            >
              {variant.title}
            </button>
          );
        })}
        <button
          type="button"
          onClick={handleClear}
          className="rounded-full border border-slate-200 px-3 py-1 text-xs font-semibold text-slate-500 transition hover:border-rose-400 hover:text-rose-500 dark:border-slate-700 dark:text-slate-300"
        >
          Clear
        </button>
      </div>
      {activeVariant?.description
        ? (
          <p className="text-sm text-slate-600 dark:text-slate-400">
            {activeVariant.description}
          </p>
        )
        : null}
      {activeVariant
          ? (
            <VariantSectionList
              sections={activeVariant.sections}
              data={data}
              errors={errors}
              onChange={onChange}
            />
          )
          : (
            <p className="text-sm text-slate-500 dark:text-slate-400">
              Select a variant to display its fields.
            </p>
          )}
    </div>
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
            {section.fields?.map((child) => (
              <FieldControl
                key={child.pointer}
                field={child}
                value={getPointerValue(data, child.pointer)}
                error={errors.get(child.pointer)}
                errors={errors}
                onChange={onChange}
                data={data}
              />
            ))}
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

function detectVariantId(
  value: JsonValue | undefined,
  variants: WebCompositeVariant[],
): string | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return undefined;
  }
  for (const variant of variants) {
    if (variantMatches(value as Record<string, JsonValue>, variant.schema)) {
      return variant.id;
    }
  }
  return undefined;
}

function variantMatches(
  value: Record<string, JsonValue>,
  schema: JsonValue,
): boolean {
  if (!schema || typeof schema !== "object" || Array.isArray(schema)) {
    return false;
  }
  const properties = (schema as { [key: string]: JsonValue }).properties as
    | Record<string, JsonValue>
    | undefined;
  if (!properties) {
    return false;
  }
  let inspected = false;
  for (const [key, spec] of Object.entries(properties)) {
    if (!spec || typeof spec !== "object") {
      continue;
    }
    if (Object.prototype.hasOwnProperty.call(spec, "const")) {
      inspected = true;
      if (!areJsonValuesEqual(value[key], (spec as any).const)) {
        return false;
      }
      continue;
    }
    if (Array.isArray((spec as any).enum)) {
      inspected = true;
      const actual = value[key];
      const matches = (spec as any).enum.some((candidate: JsonValue) =>
        areJsonValuesEqual(candidate, actual)
      );
      if (!matches) {
        return false;
      }
    }
  }
  return inspected;
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
    if (child.default_value === undefined) {
      continue;
    }
    const relative = stripPointerPrefix(child.pointer, basePointer);
    const normalized = relative || "/";
    result = setRelativeValue(result, normalized, child.default_value);
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

function describeFieldKind(kind: WebField["kind"]): string {
  switch (kind.type) {
    case "string":
    case "integer":
    case "number":
    case "boolean":
    case "json":
      return kind.type;
    case "enum":
      return `enum (${kind.options.length})`;
    case "array":
      return `array<${describeFieldKind(kind.items)}>`;
    case "composite":
      return `${
        kind.mode === "one_of" ? "oneOf" : "anyOf"
      } (${kind.variants.length})`;
    case "key_value":
      return `map<key, ${describeFieldKind(kind.value_kind)}>`;
    default:
      return "field";
  }
}
