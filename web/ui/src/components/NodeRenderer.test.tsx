import { useState } from "react";
import { describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { NodeRenderer } from "./NodeRenderer";
import { OverlayProvider } from "./Overlay";
import type { JsonValue, UiNode } from "../types";

const recursiveTreeSchema = {
  definitions: {
    treeNode: {
      type: "object",
      properties: {
        children: {
          type: "array",
          items: { $ref: "#/definitions/treeNode" },
        },
        name: { type: "string" },
        value: {
          anyOf: [
            { type: "string" },
            { type: "number" },
            { type: "boolean" },
            { type: "null" },
          ],
        },
      },
      required: ["name"],
    },
  },
  type: "object",
  properties: {
    children: {
      type: "array",
      items: { $ref: "#/definitions/treeNode" },
    },
    name: { type: "string" },
    value: {
      anyOf: [
        { type: "string" },
        { type: "number" },
        { type: "boolean" },
        { type: "null" },
      ],
    },
  },
  required: ["name"],
} as const satisfies JsonValue;

const recursiveChildrenNode: UiNode = {
  pointer: "/recursiveTree/children",
  title: null,
  description: null,
  required: false,
  default_value: [],
  kind: {
    type: "array",
    item: {
      type: "composite",
      mode: "one_of",
      allow_multiple: false,
      variants: [
        {
          id: "variant_0",
          title: "object with name",
          description: null,
          is_object: true,
          node: {
            type: "object",
            children: [],
            required: [],
          },
          schema: recursiveTreeSchema,
        },
      ],
    },
    min_items: null,
    max_items: null,
  },
};

const pointSchema = {
  type: "object",
  properties: {
    type: { const: "point" },
    value: { type: "number" },
  },
  required: ["type", "value"],
} as const satisfies JsonValue;

const vectorSchema = {
  type: "object",
  properties: {
    direction: { type: "number" },
    magnitude: { type: "number" },
    type: { const: "vector" },
  },
  required: ["direction", "magnitude", "type"],
} as const satisfies JsonValue;

const objectCellSchema = {
  type: "object",
  properties: {
    data: {
      oneOf: [pointSchema, vectorSchema],
    },
    x: { type: "number" },
    y: { type: "number" },
  },
  required: ["x", "y"],
} as const satisfies JsonValue;

const matrixNode: UiNode = {
  pointer: "/matrix",
  title: null,
  description: null,
  required: false,
  default_value: [],
  kind: {
    type: "array",
    item: {
      type: "composite",
      mode: "one_of",
      allow_multiple: false,
      variants: [
        {
          id: "variant_0",
          title: "List",
          description: null,
          is_object: false,
          node: {
            type: "array",
            item: {
              type: "composite",
              mode: "any_of",
              allow_multiple: false,
              variants: [
                {
                  id: "variant_0",
                  title: "Null",
                  description: null,
                  is_object: false,
                  node: {
                    type: "field",
                    scalar: "string",
                    enum_options: ["null"],
                    enum_values: [null],
                  },
                  schema: { type: "null" },
                },
                {
                  id: "variant_1",
                  title: "Boolean",
                  description: null,
                  is_object: false,
                  node: {
                    type: "field",
                    scalar: "boolean",
                    enum_options: null,
                    enum_values: null,
                  },
                  schema: { type: "boolean" },
                },
                {
                  id: "variant_2",
                  title: "Number",
                  description: null,
                  is_object: false,
                  node: {
                    type: "field",
                    scalar: "number",
                    enum_options: null,
                    enum_values: null,
                  },
                  schema: { type: "number" },
                },
                {
                  id: "variant_3",
                  title: "Text",
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
                  id: "variant_4",
                  title: "Object",
                  description: null,
                  is_object: true,
                  node: {
                    type: "object",
                    children: [
                      {
                        pointer: "/data",
                        title: null,
                        description: null,
                        required: false,
                        default_value: { type: "point", value: 0 },
                        kind: {
                          type: "composite",
                          mode: "one_of",
                          allow_multiple: false,
                          variants: [
                            {
                              id: "variant_0",
                              title: "point",
                              description: null,
                              is_object: true,
                              node: {
                                type: "object",
                                children: [
                                  {
                                    pointer: "/type",
                                    title: null,
                                    description: null,
                                    required: true,
                                    default_value: "point",
                                    kind: {
                                      type: "field",
                                      scalar: "string",
                                      enum_options: ["point"],
                                      enum_values: ["point"],
                                    },
                                  },
                                  {
                                    pointer: "/value",
                                    title: null,
                                    description: null,
                                    required: true,
                                    default_value: 0,
                                    kind: {
                                      type: "field",
                                      scalar: "number",
                                      enum_options: null,
                                      enum_values: null,
                                    },
                                  },
                                ],
                                required: ["type", "value"],
                              },
                              schema: pointSchema,
                            },
                            {
                              id: "variant_1",
                              title: "vector",
                              description: null,
                              is_object: true,
                              node: {
                                type: "object",
                                children: [
                                  {
                                    pointer: "/direction",
                                    title: null,
                                    description: null,
                                    required: true,
                                    default_value: 0,
                                    kind: {
                                      type: "field",
                                      scalar: "number",
                                      enum_options: null,
                                      enum_values: null,
                                    },
                                  },
                                  {
                                    pointer: "/magnitude",
                                    title: null,
                                    description: null,
                                    required: true,
                                    default_value: 0,
                                    kind: {
                                      type: "field",
                                      scalar: "number",
                                      enum_options: null,
                                      enum_values: null,
                                    },
                                  },
                                  {
                                    pointer: "/type",
                                    title: null,
                                    description: null,
                                    required: true,
                                    default_value: "vector",
                                    kind: {
                                      type: "field",
                                      scalar: "string",
                                      enum_options: ["vector"],
                                      enum_values: ["vector"],
                                    },
                                  },
                                ],
                                required: ["direction", "magnitude", "type"],
                              },
                              schema: vectorSchema,
                            },
                          ],
                        },
                      },
                      {
                        pointer: "/x",
                        title: null,
                        description: null,
                        required: true,
                        default_value: 0,
                        kind: {
                          type: "field",
                          scalar: "number",
                          enum_options: null,
                          enum_values: null,
                        },
                      },
                      {
                        pointer: "/y",
                        title: null,
                        description: null,
                        required: true,
                        default_value: 0,
                        kind: {
                          type: "field",
                          scalar: "number",
                          enum_options: null,
                          enum_values: null,
                        },
                      },
                    ],
                    required: ["x", "y"],
                  },
                  schema: objectCellSchema,
                },
              ],
            },
            min_items: null,
            max_items: null,
          },
          schema: {
            type: "array",
            items: {
              anyOf: [
                { type: "null" },
                { type: "boolean" },
                { type: "number" },
                { type: "string" },
                objectCellSchema,
              ],
            },
          },
        },
      ],
    },
    min_items: null,
    max_items: null,
  },
};

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
          schema: {
            type: "string",
            title: "Text Item",
          },
        },
        {
          id: "variant_1",
          title: "Number Item",
          description: null,
          is_object: false,
          node: {
            type: "field",
            scalar: "number",
            enum_options: null,
            enum_values: null,
          },
          schema: {
            type: "number",
            title: "Number Item",
          },
        },
        {
          id: "variant_2",
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
              {
                pointer: "/value",
                title: null,
                description: null,
                required: false,
                default_value: 0,
                kind: {
                  type: "field",
                  scalar: "number",
                  enum_options: null,
                  enum_values: null,
                },
              },
            ],
            required: ["id"],
          },
          schema: {
            type: "object",
            title: "Object Item",
            properties: {
              id: { type: "string" },
              value: { type: "number" },
            },
            required: ["id"],
          },
        },
      ],
    },
    min_items: null,
    max_items: null,
  },
};

function EditorHarness(
  { node, initialValue }: { node: UiNode; initialValue: JsonValue },
) {
  const [value, setValue] = useState<JsonValue>(initialValue);

  return (
    <OverlayProvider>
      <NodeRenderer
        node={node}
        value={value}
        errors={new Map()}
        onChange={(_pointer, nextValue) => setValue(nextValue)}
      />
      <pre data-testid="value">{JSON.stringify(value)}</pre>
    </OverlayProvider>
  );
}

describe("NodeRenderer web overlay flows", () => {
  it("supports recursive tree children through stacked overlays", async () => {
    const user = userEvent.setup();
    render(<EditorHarness node={recursiveChildrenNode} initialValue={[]} />);

    await user.click(screen.getByRole("button", { name: /add entry/i }));

    let dialog = await screen.findByRole("dialog");
    let textboxes = within(dialog).getAllByRole("textbox");
    await user.clear(textboxes[0]);
    await user.type(textboxes[0], "child");
    await user.click(within(dialog).getByRole("button", { name: /add entry/i }));

    dialog = await screen.findByRole("dialog");
    textboxes = within(dialog).getAllByRole("textbox");
    await user.clear(textboxes[0]);
    await user.type(textboxes[0], "grandchild");
    await user.click(within(dialog).getByRole("button", { name: /^done$/i }));

    dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("/children")).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: /^done$/i }));

    const value = JSON.parse(screen.getByTestId("value").textContent ?? "null");
    expect(value).toEqual([
      {
        name: "child",
        children: [
          {
            name: "grandchild",
            children: [],
            value: "",
          },
        ],
        value: "",
      },
    ]);
  });

  it("keeps parent row editor alive while editing nested matrix cells", async () => {
    const user = userEvent.setup();
    render(<EditorHarness node={matrixNode} initialValue={[]} />);

    await user.click(screen.getByRole("button", { name: /add entry/i }));

    let dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: /^edit$/i }));

    dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByText("Object"));
    expect(within(dialog).getAllByText("/data").length).toBeGreaterThan(0);
    await user.click(within(dialog).getByRole("button", { name: /^done$/i }));

    dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: /^done$/i }));

    const value = JSON.parse(screen.getByTestId("value").textContent ?? "null");
    expect(value).toEqual([
      [
        {
          x: 0,
          y: 0,
          data: {
            type: "point",
            value: 0,
          },
        },
      ],
    ]);
  });

  it("keeps complex array additions as drafts until done and preserves object variants", async () => {
    const user = userEvent.setup();
    render(<EditorHarness node={arrayOfAnyOfNode} initialValue={[]} />);

    await user.click(screen.getByRole("button", { name: /add entry/i }));
    expect(JSON.parse(screen.getByTestId("value").textContent ?? "null")).toEqual(
      [],
    );

    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByText("Object Item"));
    expect(within(dialog).queryByText("Text Item content:")).toBeNull();
    expect(within(dialog).getByText("Object Item content:")).toBeTruthy();
    expect(within(dialog).queryByText("[object Object]")).toBeNull();

    const textboxes = within(dialog).getAllByRole("textbox");
    await user.clear(textboxes[0]);
    await user.type(textboxes[0], "entry-1");
    await user.click(within(dialog).getByRole("button", { name: /^done$/i }));

    expect(JSON.parse(screen.getByTestId("value").textContent ?? "null")).toEqual(
      [
        {
          id: "entry-1",
          value: 0,
        },
      ],
    );
  });

  it("does not append a complex array entry when the dialog is cancelled", async () => {
    const user = userEvent.setup();
    render(<EditorHarness node={arrayOfAnyOfNode} initialValue={[]} />);

    await user.click(screen.getByRole("button", { name: /add entry/i }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: /^cancel$/i }));

    expect(JSON.parse(screen.getByTestId("value").textContent ?? "null")).toEqual(
      [],
    );
  });
});
