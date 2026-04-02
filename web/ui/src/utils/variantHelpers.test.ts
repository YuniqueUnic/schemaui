import { describe, expect, it } from "vitest";
import type { JsonValue, UiVariant } from "../types";
import { determineBestVariant } from "./variantHelpers";
import { variantMatchScore, variantMatches } from "./variantMatch";

const listConfigVariant: UiVariant = {
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
    title: "List Config",
    items: {
      type: "string",
    },
  },
};

const objectConfigVariant: UiVariant = {
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
    title: "Object Config",
    properties: {
      mode: {
        type: "string",
        enum: ["basic", "advanced"],
      },
      settings: {
        type: "object",
        properties: {
          timeout: { type: "integer" },
          retries: { type: "integer" },
        },
      },
    },
  },
};

describe("variant matching", () => {
  it("treats nested optional objects as valid object variant matches", () => {
    const value = {
      mode: "basic",
      settings: {},
    } satisfies JsonValue;

    expect(variantMatches(value, objectConfigVariant.schema)).toBe(true);
    expect(variantMatchScore(value, objectConfigVariant.schema)).not.toBeNull();
    expect(determineBestVariant(value, [
      listConfigVariant,
      objectConfigVariant,
    ])).toBe(objectConfigVariant);
  });

  it("keeps array values matched to the list variant", () => {
    const value = ["alpha"] satisfies JsonValue;

    expect(determineBestVariant(value, [
      listConfigVariant,
      objectConfigVariant,
    ])).toBe(listConfigVariant);
  });
});
