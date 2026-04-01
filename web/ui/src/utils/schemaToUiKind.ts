import type {
  JsonValue,
  ScalarKind,
  UiNode,
  UiNodeKind,
  UiVariant,
} from "../types";

type JsonSchema = Record<string, JsonValue>;

interface BuildContext {
  activeRefs: string[];
  definitions: Record<string, JsonValue>;
}

export function buildKindFromSchema(schemaValue: JsonValue): UiNodeKind {
  const schema = ensureObject(schemaValue);
  return visitKind(schema, {
    activeRefs: [],
    definitions: extractDefinitions(schema),
  });
}

export function materializeCompositeKind(kind: UiNodeKind): UiNodeKind {
  if (kind.type !== "composite") {
    return kind;
  }

  return {
    ...kind,
    variants: kind.variants.map((variant) => {
      const node = materializeVariantNode(variant);
      return node === variant.node ? variant : { ...variant, node };
    }),
  };
}

export function materializeVariantNode(variant: UiVariant): UiNodeKind {
  if (!needsMaterializedNode(variant)) {
    return variant.node;
  }
  return buildKindFromSchema(variant.schema);
}

function needsMaterializedNode(variant: UiVariant): boolean {
  return variant.node.type === "object" &&
    variant.node.children.length === 0 &&
    isObjectSchema(ensureObject(variant.schema));
}

function visitNode(
  schema: JsonSchema,
  pointer: string,
  required: boolean,
  context: BuildContext,
): UiNode {
  const resolved = resolveSchema(schema, context);
  if (resolved.recursive) {
    return recursiveBoundaryNode(resolved.schema, pointer, required, context);
  }

  const nextContext = resolved.ref
    ? { ...context, activeRefs: [...context.activeRefs, resolved.ref] }
    : context;

  return buildNodeFromResolvedSchema(
    resolved.schema,
    pointer,
    required,
    nextContext,
  );
}

function buildNodeFromResolvedSchema(
  schema: JsonSchema,
  pointer: string,
  required: boolean,
  context: BuildContext,
): UiNode {
  const normalized = mergeAllOf(schema, context);

  if (Array.isArray(normalized.oneOf) && normalized.oneOf.length > 0) {
    const kind = buildCompositeKind(normalized.oneOf, "one_of", context);
    return {
      pointer,
      title: readString(normalized.title),
      description: readString(normalized.description),
      required,
      default_value: defaultForKind(kind),
      kind,
    };
  }

  if (Array.isArray(normalized.anyOf) && normalized.anyOf.length > 0) {
    const kind = buildCompositeKind(normalized.anyOf, "any_of", context);
    return {
      pointer,
      title: readString(normalized.title),
      description: readString(normalized.description),
      required,
      default_value: defaultForKind(kind),
      kind,
    };
  }

  const kind = visitKind(normalized, context);
  return {
    pointer,
    title: readString(normalized.title),
    description: readString(normalized.description),
    required,
    default_value: schemaDefaultOrConst(normalized) ?? defaultForKind(kind),
    kind,
  };
}

function visitKind(schema: JsonSchema, context: BuildContext): UiNodeKind {
  const normalized = mergeAllOf(schema, context);

  if (Array.isArray(normalized.oneOf) && normalized.oneOf.length > 0) {
    return buildCompositeKind(normalized.oneOf, "one_of", context);
  }

  if (Array.isArray(normalized.anyOf) && normalized.anyOf.length > 0) {
    return buildCompositeKind(normalized.anyOf, "any_of", context);
  }

  if (isArraySchema(normalized)) {
    const item = visitArrayItemKind(normalized, context);
    return {
      type: "array",
      item,
      min_items: readNumber(normalized.minItems),
      max_items: readNumber(normalized.maxItems),
    };
  }

  if (isObjectSchema(normalized)) {
    const properties = ensureObjectMap(normalized.properties);
    const requiredFields = readStringArray(normalized.required);
    const children = Object.entries(properties).map(([name, childSchema]) =>
      visitNode(
        ensureObject(childSchema),
        appendPointer("", name),
        requiredFields.includes(name),
        context,
      )
    );

    return {
      type: "object",
      children,
      required: requiredFields,
    };
  }

  return detectScalarKind(normalized);
}

function visitArrayItemKind(schema: JsonSchema, context: BuildContext): UiNodeKind {
  const items = schema.items;
  const itemSchemaValue = Array.isArray(items) ? items[0] : items;
  const itemSchema = ensureObject(itemSchemaValue);
  const resolved = resolveSchema(itemSchema, context);

  if (resolved.recursive) {
    return normalizeEmbeddedKind(
      resolved.schema,
      recursiveBoundaryKind(resolved.schema),
      context,
    );
  }

  const nextContext = resolved.ref
    ? { ...context, activeRefs: [...context.activeRefs, resolved.ref] }
    : context;

  if (isObjectSchema(resolved.schema) && !hasCompositeSubschemas(resolved.schema)) {
    return buildSingleVariantOverlayKind(
      resolved.schema,
      visitKind(resolved.schema, nextContext),
      nextContext,
    );
  }

  return normalizeEmbeddedKind(
    resolved.schema,
    visitKind(resolved.schema, nextContext),
    nextContext,
  );
}

function buildCompositeKind(
  schemas: JsonValue[],
  mode: "one_of" | "any_of",
  context: BuildContext,
): UiNodeKind {
  return {
    type: "composite",
    mode,
    allow_multiple: false,
    variants: schemas.map((schema, index) =>
      buildVariant(ensureObject(schema), index, context)
    ),
  };
}

function buildVariant(
  schema: JsonSchema,
  index: number,
  context: BuildContext,
): UiVariant {
  const resolved = resolveSchema(schema, context);
  const nextContext = resolved.recursive || !resolved.ref
    ? context
    : { ...context, activeRefs: [...context.activeRefs, resolved.ref] };
  const node = resolved.recursive
    ? recursiveBoundaryKind(resolved.schema)
    : visitKind(resolved.schema, nextContext);
  return buildVariantFromSchema(resolved.schema, index, node, context);
}

function buildVariantFromSchema(
  schema: JsonSchema,
  index: number,
  node: UiNodeKind,
  context: BuildContext,
): UiVariant {
  return {
    id: `variant_${index}`,
    title: readString(schema.title) ?? defaultVariantTitle(index, schema),
    description: readString(schema.description),
    is_object: node.type === "object",
    node,
    schema: schemaWithDefinitions(schema, context.definitions),
  };
}

function buildSingleVariantOverlayKind(
  schema: JsonSchema,
  node: UiNodeKind,
  context: BuildContext,
): UiNodeKind {
  return {
    type: "composite",
    mode: "one_of",
    allow_multiple: false,
    variants: [buildVariantFromSchema(schema, 0, node, context)],
  };
}

function normalizeEmbeddedKind(
  schema: JsonSchema,
  kind: UiNodeKind,
  context: BuildContext,
): UiNodeKind {
  if (kind.type === "array" || kind.type === "object") {
    return buildSingleVariantOverlayKind(schema, kind, context);
  }
  return kind;
}

function recursiveBoundaryNode(
  schema: JsonSchema,
  pointer: string,
  required: boolean,
  context: BuildContext,
): UiNode {
  const kind = recursiveBoundaryKind(schema);
  return {
    pointer,
    title: readString(schema.title),
    description: readString(schema.description),
    required,
    default_value: schemaDefaultOrConst(schema) ?? defaultForKind(kind),
    kind: normalizeEmbeddedKind(schema, kind, context),
  };
}

function recursiveBoundaryKind(schema: JsonSchema): UiNodeKind {
  if (isArraySchema(schema)) {
    return {
      type: "array",
      item: {
        type: "object",
        children: [],
        required: [],
      },
      min_items: readNumber(schema.minItems),
      max_items: readNumber(schema.maxItems),
    };
  }

  if (isObjectSchema(schema)) {
    return {
      type: "object",
      children: [],
      required: [],
    };
  }

  return detectScalarKind(schema);
}

function detectScalarKind(schema: JsonSchema): UiNodeKind {
  const enumValues = readJsonArray(schema.enum);
  if (enumValues.length > 0) {
    return {
      type: "field",
      scalar: inferEnumScalar(enumValues),
      enum_options: enumValues.map(enumLabel),
      enum_values: enumValues,
    };
  }

  if (schema.const !== undefined) {
    return {
      type: "field",
      scalar: inferEnumScalar([schema.const]),
      enum_options: [enumLabel(schema.const)],
      enum_values: [schema.const],
    };
  }

  if (normalizeType(schema.type) === "null") {
    return {
      type: "field",
      scalar: "string",
      enum_options: ["null"],
      enum_values: [null],
    };
  }

  return {
    type: "field",
    scalar: normalizeScalarKind(schema),
    enum_options: null,
    enum_values: null,
  };
}

function normalizeScalarKind(schema: JsonSchema): ScalarKind {
  switch (normalizeType(schema.type)) {
    case "integer":
      return "integer";
    case "number":
      return "number";
    case "boolean":
      return "boolean";
    case "string":
    default:
      return "string";
  }
}

function defaultForKind(kind: UiNodeKind): JsonValue {
  switch (kind.type) {
    case "field":
      if (kind.enum_values?.length) {
        return clone(kind.enum_values[0]);
      }
      switch (kind.scalar) {
        case "integer":
        case "number":
          return 0;
        case "boolean":
          return false;
        case "string":
        default:
          return "";
      }
    case "array":
      return [];
    case "object": {
      const result: Record<string, JsonValue> = {};
      for (const child of kind.children) {
        const key = pointerSegment(child.pointer);
        if (!key) {
          continue;
        }
        result[key] = clone(child.default_value ?? defaultForKind(child.kind));
      }
      return result;
    }
    case "composite":
      if (kind.allow_multiple) {
        return [];
      }
      return kind.variants[0] ? generateVariantDefault(kind.variants[0]) : {};
  }
}

function generateVariantDefault(variant: UiVariant): JsonValue {
  const node = variant.node;
  if (node.type === "object") {
    const result: Record<string, JsonValue> = {};
    const schemaProps = ensureObjectMap(ensureObject(variant.schema).properties);
    for (const [key, value] of Object.entries(schemaProps)) {
      const propertySchema = ensureObject(value);
      if (propertySchema.const !== undefined) {
        result[key] = clone(propertySchema.const);
      }
    }

    for (const child of node.children) {
      const key = pointerSegment(child.pointer);
      if (!key || key in result) {
        continue;
      }
      if (node.required.includes(key) || child.default_value !== undefined) {
        result[key] = clone(child.default_value ?? defaultForKind(child.kind));
      }
    }

    return result;
  }

  if (node.type === "array") {
    return [defaultForKind(node.item)];
  }

  return defaultForKind(node);
}

function schemaDefaultOrConst(schema: JsonSchema): JsonValue | undefined {
  if (schema.default !== undefined) {
    return clone(schema.default);
  }
  if (schema.const !== undefined) {
    return clone(schema.const);
  }
  return undefined;
}

function resolveSchema(
  schema: JsonSchema,
  context: BuildContext,
): { schema: JsonSchema; ref?: string; recursive: boolean } {
  const ref = readString(schema.$ref);
  if (!ref || !ref.startsWith("#/")) {
    return { schema, recursive: false };
  }

  const resolved = resolveLocalRef(ref, context.definitions);
  return {
    schema: resolved,
    ref,
    recursive: context.activeRefs.includes(ref),
  };
}

function resolveLocalRef(
  ref: string,
  definitions: Record<string, JsonValue>,
): JsonSchema {
  const segments = ref.slice(2).split("/");
  let current: JsonValue = { definitions };
  for (const raw of segments) {
    const key = raw.replace(/~1/g, "/").replace(/~0/g, "~");
    const next: JsonValue | undefined = ensureObject(current)[key];
    if (next === undefined) {
      return {};
    }
    current = next;
  }
  return ensureObject(current);
}

function schemaWithDefinitions(
  schema: JsonSchema,
  definitions: Record<string, JsonValue>,
): JsonValue {
  const next = clone(schema) as JsonSchema;
  if (!next.definitions && Object.keys(definitions).length > 0) {
    next.definitions = clone(definitions);
  }
  return next;
}

function extractDefinitions(schema: JsonSchema): Record<string, JsonValue> {
  const definitions = ensureObjectMap(schema.definitions);
  const dollarDefs = ensureObjectMap(schema.$defs);
  return { ...dollarDefs, ...definitions };
}

function mergeAllOf(schema: JsonSchema, context: BuildContext): JsonSchema {
  if (!Array.isArray(schema.allOf) || schema.allOf.length === 0) {
    return schema;
  }

  return schema.allOf.reduce<JsonSchema>((acc, entry) => {
    const resolved = resolveSchema(ensureObject(entry), context);
    return deepMerge(acc, resolved.schema);
  }, omitKey(schema, "allOf"));
}

function deepMerge(left: JsonSchema, right: JsonSchema): JsonSchema {
  const result: JsonSchema = clone(left) as JsonSchema;
  for (const [key, value] of Object.entries(right)) {
    const existing = result[key];
    if (isPlainObject(existing) && isPlainObject(value)) {
      result[key] = deepMerge(
        existing as JsonSchema,
        value as JsonSchema,
      );
    } else {
      result[key] = clone(value);
    }
  }
  return result;
}

function isObjectSchema(schema: JsonSchema): boolean {
  return normalizeType(schema.type) === "object" ||
    Object.keys(ensureObjectMap(schema.properties)).length > 0;
}

function isArraySchema(schema: JsonSchema): boolean {
  return normalizeType(schema.type) === "array" || schema.items !== undefined;
}

function hasCompositeSubschemas(schema: JsonSchema): boolean {
  return Array.isArray(schema.oneOf) && schema.oneOf.length > 0 ||
    Array.isArray(schema.anyOf) && schema.anyOf.length > 0;
}

function normalizeType(typeValue: JsonValue | undefined): string | undefined {
  if (typeof typeValue === "string") {
    return typeValue;
  }
  if (Array.isArray(typeValue)) {
    return typeValue.find((entry) =>
      typeof entry === "string" && entry !== "null"
    ) as string | undefined;
  }
  return undefined;
}

function readString(value: JsonValue | undefined): string | null | undefined {
  return typeof value === "string" ? value : undefined;
}

function readNumber(value: JsonValue | undefined): number | null | undefined {
  return typeof value === "number" ? value : undefined;
}

function readStringArray(value: JsonValue | undefined): string[] {
  return Array.isArray(value)
    ? value.filter((entry): entry is string => typeof entry === "string")
    : [];
}

function readJsonArray(value: JsonValue | undefined): JsonValue[] {
  return Array.isArray(value) ? value.map((entry) => clone(entry)) : [];
}

function ensureObject(value: JsonValue | undefined): JsonSchema {
  return isPlainObject(value) ? (value as JsonSchema) : {};
}

function ensureObjectMap(value: JsonValue | undefined): Record<string, JsonValue> {
  return isPlainObject(value) ? (value as Record<string, JsonValue>) : {};
}

function isPlainObject(value: JsonValue | undefined): value is Record<string, JsonValue> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function appendPointer(base: string, segment: string): string {
  const encoded = segment.replace(/~/g, "~0").replace(/\//g, "~1");
  return `${base}/${encoded}`;
}

function pointerSegment(pointer: string): string | undefined {
  if (!pointer || pointer === "/") {
    return undefined;
  }
  const parts = pointer.split("/").filter(Boolean);
  const last = parts[parts.length - 1];
  return last?.replace(/~1/g, "/").replace(/~0/g, "~");
}

function omitKey(schema: JsonSchema, key: string): JsonSchema {
  const result = { ...schema };
  delete result[key];
  return result;
}

function enumLabel(value: JsonValue): string {
  if (typeof value === "string" || typeof value === "number" ||
    typeof value === "boolean") {
    return String(value);
  }
  return JSON.stringify(value);
}

function inferEnumScalar(values: JsonValue[]): ScalarKind {
  let inferred: ScalarKind | null = null;
  for (const value of values) {
    const next = typeof value === "number"
      ? (Number.isInteger(value) ? "integer" : "number")
      : typeof value === "boolean"
      ? "boolean"
      : "string";

    if (inferred && inferred !== next) {
      return "string";
    }
    inferred = next;
  }
  return inferred ?? "string";
}

function defaultVariantTitle(index: number, schema: JsonSchema): string {
  const ref = readString(schema.$ref);
  if (ref) {
    const name = ref.split("/").pop();
    if (name) {
      return humanize(name);
    }
  }

  const properties = ensureObjectMap(schema.properties);
  for (const key of ["type", "kind"]) {
    const propertySchema = ensureObject(properties[key]);
    if (typeof propertySchema.const === "string") {
      return propertySchema.const;
    }
  }

  if (isArraySchema(schema)) {
    return "List";
  }
  if (isObjectSchema(schema)) {
    return "Object";
  }

  const type = normalizeType(schema.type);
  return type ? humanize(type) : `Variant ${index + 1}`;
}

function humanize(value: string): string {
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[_-]+/g, " ")
    .replace(/^\w/, (match) => match.toUpperCase());
}

function clone<T extends JsonValue>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
