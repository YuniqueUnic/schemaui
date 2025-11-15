import type { JsonValue, WebBlueprint, WebField, WebSection } from '../types';
import { mergeDefaults } from './jsonPointer';

export function applyBlueprintDefaults(
  blueprint: WebBlueprint | undefined,
  data: JsonValue,
): JsonValue {
  if (!blueprint) {
    return data;
  }
  const defaults: Record<string, JsonValue | undefined> = {};
  blueprint.roots.forEach((root) => {
    root.sections?.forEach((section) => collectSectionDefaults(section, defaults));
  });
  return mergeDefaults(data, defaults);
}

function collectSectionDefaults(
  section: WebSection,
  defaults: Record<string, JsonValue | undefined>,
) {
  section.fields?.forEach((field) => recordDefault(field, defaults));
  section.sections?.forEach((child) => collectSectionDefaults(child, defaults));
}

function recordDefault(
  field: WebField,
  defaults: Record<string, JsonValue | undefined>,
) {
  if (field.default_value !== undefined && field.pointer) {
    defaults[field.pointer] = field.default_value;
  }
}
