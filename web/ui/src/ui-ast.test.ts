import { describe, expect, it } from "vitest";
import type { UiAst, UiVariant } from "./types";
import { applyUiDefaults, variantDefault } from "./ui-ast";

const ast: UiAst = {
  roots: [
    {
      pointer: "/complexComposite",
      title: "10. Complex Composite",
      description: null,
      required: true,
      default_value: {},
      kind: {
        type: "object",
        children: [
          {
            pointer: "/complexComposite/config",
            title: "complexComposite",
            description: null,
            required: true,
            default_value: [],
            kind: {
              type: "composite",
              mode: "any_of",
              allow_multiple: false,
              variants: [
                {
                  id: "variant_0",
                  title: "List Config",
                  description: null,
                  is_object: false,
                  node: {
                    type: "array",
                    item: {
                      type: "field",
                      scalar: "string",
                      enum_options: null,
                      enum_values: null,
                    },
                    min_items: null,
                    max_items: null,
                  },
                  schema: {
                    type: "array",
                    items: { type: "string" },
                  },
                },
                {
                  id: "variant_1",
                  title: "Object Config",
                  description: null,
                  is_object: true,
                  node: {
                    type: "object",
                    children: [],
                    required: [],
                  },
                  schema: {
                    type: "object",
                    properties: {
                      mode: { type: "string" },
                    },
                  },
                },
              ],
            },
          },
        ],
        required: ["config"],
      },
    },
  ],
};

describe("applyUiDefaults", () => {
  it("keeps composite defaults from the active/default variant instead of the last variant", () => {
    expect(applyUiDefaults(ast, {})).toEqual({
      complexComposite: {
        config: [],
      },
    });
  });

  it("seeds only missing required fields and preserves null and absent optional fields", () => {
    const roundTripAst: UiAst = {
      roots: [
        {
          pointer: "/requiredHost",
          title: "Host",
          description: null,
          required: true,
          default_value: "localhost",
          kind: {
            type: "field",
            scalar: "string",
            enum_options: null,
            enum_values: null,
          },
        },
        {
          pointer: "/optionalNote",
          title: "Note",
          description: null,
          required: false,
          default_value: "should-not-be-seeded",
          kind: {
            type: "field",
            scalar: "string",
            enum_options: null,
            enum_values: null,
          },
        },
        {
          pointer: "/nullableToken",
          title: "Token",
          description: null,
          required: true,
          default_value: "fallback-token",
          kind: {
            type: "field",
            scalar: "string",
            enum_options: null,
            enum_values: null,
            nullable: true,
          },
        },
        {
          pointer: "/optionalSection",
          title: "Optional Section",
          description: null,
          required: false,
          default_value: {},
          kind: {
            type: "object",
            required: ["innerRequired"],
            children: [
              {
                pointer: "/optionalSection/innerRequired",
                title: "Inner Required",
                description: null,
                required: true,
                default_value: "inner-default",
                kind: {
                  type: "field",
                  scalar: "string",
                  enum_options: null,
                  enum_values: null,
                },
              },
            ],
          },
        },
      ],
    };

    expect(
      applyUiDefaults(roundTripAst, {
        nullableToken: null,
      }),
    ).toEqual({
      requiredHost: "localhost",
      nullableToken: null,
    });
  });
});

describe("variantDefault", () => {
  it("populates const, required, and $ref-authored defaults while omitting optional placeholders", () => {
    const variant: UiVariant = {
      id: "variant_0",
      title: "Webhook",
      description: null,
      is_object: true,
      node: {
        type: "object",
        children: [],
        required: ["kind", "endpoint"],
      },
      schema: {
        type: "object",
        required: ["kind", "endpoint"],
        properties: {
          kind: { const: "webhook" },
          endpoint: { type: "string" },
          retries: { $ref: "#/$defs/Retries" },
          optionalSecret: { type: "string" },
        },
        $defs: {
          Retries: {
            type: "integer",
            default: 3,
          },
        },
      },
    };

    expect(variantDefault(variant)).toEqual({
      kind: "webhook",
      endpoint: "",
      retries: 3,
    });
  });
});
