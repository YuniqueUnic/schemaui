/**
 * Variant type configurations for default value generation
 * This makes variant identification configurable and extensible
 */

export interface VariantConfig {
    /** Properties that must be present */
    requiredProperties: string[];
    /** Properties that must NOT be present */
    excludedProperties: string[];
    /** Default values for this variant type */
    defaults: Record<string, unknown>;
}

/**
 * Predefined variant configurations
 * Extend this object to add new variant types
 */
export const VARIANT_CONFIGS: Record<string, VariantConfig> = {
    simpleItem: {
        requiredProperties: ["label", "enabled"],
        excludedProperties: ["values", "url"],
        defaults: {
            id: 1001,
            label: "item",
            enabled: false,
        },
    },
    numericItem: {
        requiredProperties: ["values"],
        excludedProperties: ["label", "enabled"],
        defaults: {
            id: 2001,
            values: [1],
        },
    },
    target: {
        requiredProperties: ["url"],
        excludedProperties: [],
        defaults: {
            url: "https://example.com",
            priority: 5,
            active: false,
        },
    },
};

/**
 * Field-specific default values
 */
export const FIELD_DEFAULTS: Record<string, unknown> = {
    id: 0,
    label: "",
    enabled: false,
    values: [],
    url: "",
    priority: 1,
    active: false,
};
