import type { JsonValue, UiVariant } from "../types";
import {
  VARIANT_CONFIGS,
  type VariantConfig,
} from "../constants/variantDefaults";
import { deepEqual } from "./deepEqual";
import { variantMatchScore } from "./variantMatch";
import { variantDefault } from "../ui-ast";

/**
 * Variant identification and matching utilities
 */

/**
 * Identifies the variant type based on schema properties
 */
export function identifyVariantType(
  schemaProperties: Record<string, unknown>,
): string | null {
  for (const [typeName, config] of Object.entries(VARIANT_CONFIGS)) {
    if (matchesVariantConfig(schemaProperties, config)) {
      return typeName;
    }
  }
  return null;
}

/**
 * Checks if schema properties match a variant configuration
 */
function matchesVariantConfig(
  schemaProperties: Record<string, unknown>,
  config: VariantConfig,
): boolean {
  // Check required properties are present
  const hasRequired = config.requiredProperties.some(
    (prop) => prop in schemaProperties,
  );
  if (!hasRequired) return false;

  // Check excluded properties are absent
  const hasExcluded = config.excludedProperties.some(
    (prop) => prop in schemaProperties,
  );
  if (hasExcluded) return false;

  return true;
}

/**
 * Determines which variant matches the given value
 */
export function determineVariant(
  value: JsonValue | undefined,
  variants: UiVariant[],
): UiVariant | undefined {
  return determineBestVariant(value, variants);
}

/**
 * Determines the best matching variant, handling ambiguous cases
 * When multiple variants match, uses exact default value matching to disambiguate
 */
export function determineBestVariant(
  value: JsonValue | undefined,
  variants: UiVariant[],
): UiVariant {
  const matchingVariants = variants
    .map((variant) => ({
      variant,
      score: variantMatchScore(value, variant.schema),
    }))
    .filter((entry): entry is { variant: UiVariant; score: number } =>
      entry.score !== null
    );

  // Exactly one match - use it
  if (matchingVariants.length === 1) {
    return matchingVariants[0].variant;
  }

  // Multiple matches - try exact default matching
  if (matchingVariants.length > 1) {
    const exactMatch = matchingVariants.find(({ variant }) => {
      const defaultVal = variantDefault(variant);
      return deepEqual(value, defaultVal);
    });
    if (exactMatch) {
      return exactMatch.variant;
    }
    return matchingVariants
      .sort((left, right) => right.score - left.score)[0].variant;
  }

  // No matches - fallback to first variant
  return variants[0];
}

/**
 * Extracts schema properties from a variant schema
 */
export function extractSchemaProperties(
  schema: JsonValue | undefined,
): Record<string, unknown> {
  if (
    schema &&
    typeof schema === "object" &&
    !Array.isArray(schema) &&
    "properties" in schema
  ) {
    return schema.properties as Record<string, unknown>;
  }
  return {};
}
