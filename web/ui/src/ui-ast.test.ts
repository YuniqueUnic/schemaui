import { describe, expect, it } from "vitest";
import type { UiAst } from "./types";
import { applyUiDefaults } from "./ui-ast";

const ast: UiAst = {
  roots: [
    {
      pointer: "/complexComposite",
      title: "10. Complex Composite",
      description: null,
      required: false,
      default_value: {},
      kind: {
        type: "object",
        children: [
          {
            pointer: "/complexComposite/config",
            title: "complexComposite",
            description: null,
            required: false,
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
        required: [],
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
});
