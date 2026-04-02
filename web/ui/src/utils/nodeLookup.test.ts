import { describe, expect, it } from "vitest";
import type { UiNode } from "../types";
import { findNodeByPointer, resolveNavigablePointer } from "./nodeLookup";

const arrayOfAnyOfNode: UiNode = {
  pointer: "/arrayOfAnyOf",
  title: "5. Array of anyOf items",
  description: null,
  required: false,
  default_value: [],
  kind: {
    type: "array",
    item: {
      type: "composite",
      mode: "any_of",
      allow_multiple: false,
      variants: [
        {
          id: "variant_0",
          title: "Text Item",
          description: null,
          is_object: false,
          node: {
            type: "field",
            scalar: "string",
            enum_options: null,
            enum_values: null,
          },
          schema: { type: "string" },
        },
        {
          id: "variant_1",
          title: "Object Item",
          description: null,
          is_object: true,
          node: {
            type: "object",
            children: [
              {
                pointer: "/id",
                title: null,
                description: null,
                required: true,
                default_value: "",
                kind: {
                  type: "field",
                  scalar: "string",
                  enum_options: null,
                  enum_values: null,
                },
              },
            ],
            required: ["id"],
          },
          schema: {
            type: "object",
            properties: { id: { type: "string" } },
            required: ["id"],
          },
        },
      ],
    },
    min_items: null,
    max_items: null,
  },
};

describe("nodeLookup", () => {
  it("builds a synthetic node for dynamic array entry pointers", () => {
    const resolved = findNodeByPointer([arrayOfAnyOfNode], "/arrayOfAnyOf/1");

    expect(resolved?.pointer).toBe("/arrayOfAnyOf/1");
    expect(resolved?.kind.type).toBe("composite");
    expect(resolved?.title).toBe("5. Array of anyOf items entry 2");
  });

  it("falls back to the nearest navigable array entry pointer", () => {
    expect(
      resolveNavigablePointer([arrayOfAnyOfNode], "/arrayOfAnyOf/1/id"),
    ).toBe("/arrayOfAnyOf/1");
  });
});
