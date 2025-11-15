import { Fragment, memo, useMemo } from 'react';
import type { JsonValue, WebField, WebSection } from '../types';
import { getPointerValue } from '../utils/jsonPointer';
import { ChevronRight } from 'lucide-react';

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
        <div className="h-6 w-56 animate-pulse rounded-full bg-slate-800/50" />
        <div className="space-y-4">
          {Array.from({ length: 5 }).map((_, index) => (
            <div
              key={`skeleton-${index}`}
              className="h-28 animate-pulse rounded-2xl bg-slate-800/40"
            />
          ))}
        </div>
      </div>
    );
  }

  if (!section || (!section.fields?.length && !section.sections?.length)) {
    return (
      <div className="flex h-full flex-col items-center justify-center text-center text-sm text-slate-400">
        <p>No fields in this section.</p>
        <p className="text-xs text-slate-500">Select another node from the tree.</p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-4 overflow-auto px-8 py-6">
      {breadcrumbs.length ? (
        <nav className="flex flex-wrap items-center gap-2 text-xs text-slate-500">
          {breadcrumbs.map((crumb, index) => (
            <Fragment key={`${crumb}-${index}`}>
              <span className="rounded-full bg-slate-900/70 px-3 py-1 text-slate-300">
                {crumb}
              </span>
              {index < breadcrumbs.length - 1 ? (
                <ChevronRight className="h-3.5 w-3.5 text-slate-600" />
              ) : null}
            </Fragment>
          ))}
        </nav>
      ) : null}
      <article className="rounded-2xl border border-slate-800/70 bg-slate-900/40 p-6 shadow-shell">
        <header>
          <p className="text-xs uppercase tracking-[0.25em] text-slate-500">Section</p>
          <h2 className="text-2xl font-semibold text-white">{section.title}</h2>
          {section.description ? (
            <p className="mt-2 text-sm text-slate-400">{section.description}</p>
          ) : null}
        </header>
        <div className="mt-6 space-y-5">
          {section.fields?.map((field) => (
            <FieldControl
              key={field.pointer}
              field={field}
              value={getPointerValue(data, field.pointer)}
              error={errors.get(field.pointer)}
              onChange={onChange}
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
  onChange: (pointer: string, value: JsonValue) => void;
}

function FieldControl({ field, value, error, onChange }: FieldControlProps) {
  const id = field.pointer || field.name;
  const label = field.label || field.name;
  const description = field.description;
  const required = field.required;
  const pointer = field.pointer;

  const body = useMemo(() => {
    if (!pointer) {
      return null;
    }
    switch (field.kind?.type) {
      case 'string':
        return (
          <input
            id={id}
            type="text"
            className="rounded-xl border border-slate-700/70 bg-slate-900/40 px-3 py-2 text-sm text-slate-100 outline-none ring-brand-400/50 focus:border-brand-400 focus:ring-2"
            value={(value as string) ?? ''}
            onChange={(event) => onChange(pointer, event.target.value)}
            spellCheck={false}
          />
        );
      case 'integer':
      case 'number': {
        const parsedValue =
          typeof value === 'number'
            ? value
            : typeof value === 'string'
              ? Number(value)
              : '';
        return (
          <input
            id={id}
            type="number"
            className="rounded-xl border border-slate-700/70 bg-slate-900/40 px-3 py-2 text-sm text-slate-100 outline-none ring-brand-400/50 focus:border-brand-400 focus:ring-2"
            value={Number.isNaN(parsedValue) ? '' : parsedValue}
            onChange={(event) => {
              const next = event.target.value;
              onChange(pointer, next === '' ? null : Number(next));
            }}
          />
        );
      }
      case 'boolean':
        return (
          <label className="inline-flex items-center gap-3">
            <input
              id={id}
              type="checkbox"
              className="h-5 w-5 accent-brand-400"
              checked={Boolean(value)}
              onChange={(event) => onChange(pointer, event.target.checked)}
            />
            <span className="text-sm text-slate-200">Enabled</span>
          </label>
        );
      case 'enum':
        return (
          <select
            id={id}
            className="w-full rounded-xl border border-slate-700/70 bg-slate-900/40 px-3 py-2 text-sm text-slate-100 outline-none ring-brand-400/50 focus:border-brand-400 focus:ring-2"
            value={(value as string) ?? field.kind.options?.[0] ?? ''}
            onChange={(event) => onChange(pointer, event.target.value)}
          >
            {(field.kind.options || []).map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        );
      case 'array':
        return (
          <PrimitiveArrayEditor
            pointer={pointer}
            itemKind={field.kind.items?.type}
            value={Array.isArray(value) ? value : []}
            onChange={onChange}
          />
        );
      default:
        return (
          <textarea
            id={id}
            rows={6}
            className="rounded-2xl border border-slate-700/70 bg-slate-900/40 px-3 py-2 font-mono text-sm text-slate-100 outline-none ring-brand-400/50 focus:border-brand-400 focus:ring-2"
            value={value ? JSON.stringify(value, null, 2) : ''}
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
  }, [field.kind, id, onChange, pointer, value]);

  return (
    <section className="rounded-2xl border border-slate-800/60 bg-slate-950/30 p-4">
      <div className="flex items-center gap-2">
        <label htmlFor={id} className="text-sm font-medium text-white">
          {label} {required ? <span className="text-rose-300">*</span> : null}
        </label>
        <span className="font-mono text-[10px] text-slate-500">{pointer}</span>
      </div>
      {description ? (
        <p className="mt-1 text-xs text-slate-400">{description}</p>
      ) : null}
      <div className="mt-3">{body}</div>
      {error ? <p className="mt-2 text-xs text-rose-300">{error}</p> : null}
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
  itemKind = 'string',
  value,
  onChange,
}: PrimitiveArrayEditorProps) {
  const handleChange = (index: number, next: JsonValue) => {
    const copy = [...value];
    copy[index] = next;
    onChange(pointer, copy);
  };

  const addItem = () => {
    const defaults: Record<string, JsonValue> = {
      string: '',
      number: 0,
      integer: 0,
      boolean: false,
    };
    const placeholder = defaults[itemKind] ?? null;
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
              type={itemKind === 'boolean' ? 'text' : itemKind === 'number' || itemKind === 'integer' ? 'number' : 'text'}
              className="flex-1 rounded-xl border border-slate-700/70 bg-slate-900/40 px-3 py-2 text-sm text-slate-100 outline-none focus:border-brand-400"
              value={
                typeof entry === 'string' || typeof entry === 'number'
                  ? String(entry)
                  : entry === null
                    ? ''
                    : JSON.stringify(entry)
              }
              onChange={(event) => {
                if (itemKind === 'number' || itemKind === 'integer') {
                  const num = Number(event.target.value);
                  handleChange(index, Number.isNaN(num) ? null : num);
                } else if (itemKind === 'boolean') {
                  handleChange(index, event.target.value === 'true');
                } else {
                  handleChange(index, event.target.value);
                }
              }}
            />
            <button
              type="button"
              onClick={() => removeItem(index)}
              className="rounded-full border border-slate-700 px-3 py-1 text-xs text-slate-300 transition hover:border-rose-400 hover:text-rose-300"
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
