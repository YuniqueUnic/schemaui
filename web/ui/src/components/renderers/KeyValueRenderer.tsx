import type { JsonValue, UiNode } from "../../types";
import { defaultForKind } from "../../ui-ast";
import { formatValueSummary } from "../../utils/typeHelpers";
import { buildKindFromSchema, materializeCompositeKind } from "../../utils/schemaToUiKind";
import { useOverlay } from "../Overlay";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { EntryEditor } from "./shared/EntryEditor";

type KeyValueNode = UiNode & {
  kind: Extract<import("../../types").UiNodeKind, { type: "key_value" }>;
};

interface KeyValueRendererProps {
  node: KeyValueNode;
  value: JsonValue | undefined;
  errors: Map<string, string>;
  onChange: (pointer: string, value: JsonValue) => void;
  renderNode: (
    node: UiNode,
    value: JsonValue | undefined,
    errors: Map<string, string>,
    onChange: (pointer: string, value: JsonValue) => void,
  ) => React.ReactNode;
}

interface KeyValueEntryDraft {
  key: string;
  value: JsonValue;
}

export function KeyValueRenderer({
  node,
  value,
  errors,
  onChange,
  renderNode,
}: KeyValueRendererProps) {
  const overlay = useOverlay();
  const entries = Object.entries(
    value && typeof value === "object" && !Array.isArray(value) ? value : {},
  ) as Array<[string, JsonValue]>;
  const template = node.kind.template;

  const editKind = materializeCompositeKind(template.value_kind);
  const keyKind = buildKindFromSchema(template.key_schema);

  const entryNodeFor = (entryIndex: number): UiNode => ({
    pointer: `${node.pointer}/${entryIndex}`,
    title: node.title
      ? `${node.title} entry ${entryIndex + 1}`
      : `Entry ${entryIndex + 1}`,
    description: node.description,
    required: false,
    default_value: draftToJsonValue(buildDraftValue(template)),
    kind: {
      type: "object",
      required: ["key"],
      children: [
        {
          pointer: "/key",
          title: template.key_title,
          description: template.key_description,
          required: true,
          default_value: template.key_default ?? "",
          kind: keyKind,
        },
        {
          pointer: "/value",
          title: template.value_title,
          description: template.value_description,
          required: false,
          default_value: template.value_default ?? defaultForKind(editKind),
          kind: editKind,
        },
      ],
    },
  });

  const openEditor = (
    entryIndex: number,
    initialValue: KeyValueEntryDraft,
    onSaveEntry: (draft: KeyValueEntryDraft) => void,
  ) => {
    const entryNode = entryNodeFor(entryIndex);

    overlay.open({
      title: `${node.title ?? node.pointer} · Entry ${entryIndex + 1}`,
      content: (close) => (
        <EntryEditor
          node={entryNode}
          initialValue={draftToJsonValue(initialValue)}
          errors={errors}
          onSave={(newValue) => {
            const draft = parseDraftValue(newValue, template);
            onSaveEntry(draft);
            close();
          }}
          onClose={close}
          renderNode={renderNode}
        />
      ),
    });
  };

  const saveEntry = (
    previousKey: string | null,
    draft: KeyValueEntryDraft,
  ) => {
    const next = {
      ...(value && typeof value === "object" && !Array.isArray(value) ? value : {}),
    } as Record<string, JsonValue>;

    if (previousKey !== null) {
      delete next[previousKey];
    }
    next[draft.key] = draft.value;
    onChange(node.pointer, next);
  };

  return (
    <div className="space-y-2">
      {entries.length === 0
        ? (
          <div className="text-center py-6 text-muted-foreground border border-dashed rounded-lg">
            <p className="text-sm">No entries yet</p>
            <p className="text-xs mt-1">
              Click below to add your first entry
            </p>
          </div>
        )
        : (
          entries.map(([key, entryValue], index) => (
            <Card
              key={`${node.pointer}-${key}-${index}`}
              className="px-3 py-2"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 flex-1 space-y-1">
                  <div className="flex items-center gap-2 min-w-0">
                    <Badge variant="secondary" className="shrink-0">
                      {index + 1}
                    </Badge>
                    <span className="text-sm font-medium truncate">
                      {key}
                    </span>
                  </div>
                  <p className="text-xs text-muted-foreground break-words">
                    {formatValueSummary(entryValue)}
                  </p>
                </div>
                <div className="flex gap-1 shrink-0">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() =>
                      openEditor(
                        index,
                        { key, value: entryValue },
                        (draft) => saveEntry(key, draft),
                      )}
                  >
                    Edit
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      const next = { ...(value as Record<string, JsonValue> ?? {}) };
                      delete next[key];
                      onChange(node.pointer, next);
                    }}
                    className="text-destructive hover:text-destructive"
                  >
                    Remove
                  </Button>
                </div>
              </div>
            </Card>
          ))
        )}
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={() =>
          openEditor(entries.length, buildDraftValue(template), (draft) =>
            saveEntry(null, draft)
          )}
        className="w-full"
      >
        + Add entry
      </Button>
    </div>
  );
}

function buildDraftValue(
  template: KeyValueNode["kind"]["template"],
): KeyValueEntryDraft {
  return {
    key: typeof template.key_default === "string" ? template.key_default : "",
    value: template.value_default ?? defaultForKind(template.value_kind),
  };
}

function parseDraftValue(
  value: JsonValue,
  template: KeyValueNode["kind"]["template"],
): KeyValueEntryDraft {
  const draft = value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, JsonValue>
    : {};
  const nextKey = draft.key;
  return {
    key: typeof nextKey === "string"
      ? nextKey
      : typeof template.key_default === "string"
      ? template.key_default
      : "",
    value: draft.value ?? template.value_default ?? defaultForKind(template.value_kind),
  };
}

function draftToJsonValue(draft: KeyValueEntryDraft): JsonValue {
  return {
    key: draft.key,
    value: draft.value,
  };
}
