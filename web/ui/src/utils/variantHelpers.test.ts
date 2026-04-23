import { describe, expect, it } from "vitest";
import type { JsonValue, UiVariant } from "../types";
import { determineBestVariant } from "./variantHelpers";
import { variantMatchScore, variantMatches } from "./variantMatch";
import { variantDefault } from "../ui-ast";

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

const optionalObjectVariant: UiVariant = {
  id: "variant_0",
  title: "Speedtest configuration",
  description: null,
  is_object: true,
  node: {
    type: "object",
    children: [],
    required: [],
  },
  schema: {
    $ref: "#/$defs/SpeedtestConfig",
    $defs: {
      SpeedtestConfig: {
        type: "object",
        title: "Speedtest configuration",
        properties: {
          "speedtest-target": {
            type: "string",
            default: "127.0.0.1:8080",
          },
          "expected-rate-mbps": {
            type: "integer",
            default: 5,
          },
        },
      },
    },
  },
};

const nullVariant: UiVariant = {
  id: "variant_1",
  title: "Null",
  description: null,
  is_object: false,
  node: {
    type: "field",
    scalar: "string",
    enum_options: ["null"],
    enum_values: [null],
    nullable: true,
  },
  schema: { type: "null" },
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

  it("materializes $defs-backed object defaults for object-or-null variants", () => {
    expect(variantDefault(optionalObjectVariant)).toEqual({
      "speedtest-target": "127.0.0.1:8080",
      "expected-rate-mbps": 5,
    });
  });

  it("does not let the null schema match non-null values", () => {
    const value = variantDefault(optionalObjectVariant);

    expect(variantMatches(value, nullVariant.schema)).toBe(false);
    expect(variantMatchScore(value, nullVariant.schema)).toBeNull();
    expect(determineBestVariant(value, [
      optionalObjectVariant,
      nullVariant,
    ])).toBe(optionalObjectVariant);
  });
});
