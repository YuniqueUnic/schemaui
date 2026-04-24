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

const complexCompositeConfigNode: UiNode = {
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
          title: "List Config",
          items: {
            type: "string",
          },
        },
      },
      {
        id: "variant_1",
        title: "Object Config",
        description: null,
        is_object: true,
        node: {
          type: "object",
          children: [
            {
              pointer: "/mode",
              title: null,
              description: null,
              required: false,
              default_value: "basic",
              kind: {
                type: "field",
                scalar: "string",
                enum_options: ["basic", "advanced"],
                enum_values: ["basic", "advanced"],
              },
            },
            {
              pointer: "/settings",
              title: null,
              description: null,
              required: false,
              default_value: {},
              kind: {
                type: "object",
                children: [
                  {
                    pointer: "/timeout",
                    title: null,
                    description: null,
                    required: false,
                    default_value: 0,
                    kind: {
                      type: "field",
                      scalar: "integer",
                      enum_options: null,
                      enum_values: null,
                    },
                  },
                  {
                    pointer: "/retries",
                    title: null,
                    description: null,
                    required: false,
                    default_value: 0,
                    kind: {
                      type: "field",
                      scalar: "integer",
                      enum_options: null,
                      enum_values: null,
                    },
                  },
                ],
                required: [],
              },
            },
          ],
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
      },
    ],
  },
};

const speedtestCompositeNode: UiNode = {
  pointer: "/multipath/speedtest",
  title: "Speedtest configuration",
  description: null,
  required: false,
  default_value: {},
  kind: {
    type: "composite",
    mode: "any_of",
    allow_multiple: false,
    variants: [
      {
        id: "variant_0",
        title: "Object",
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
      },
      {
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
      },
    ],
  },
};

const dnsHostsNode: UiNode = {
  pointer: "/dns/hosts",
  title: "hosts",
  description: "www.google.com: 8.8.8.8",
  required: false,
  default_value: {},
  kind: {
    type: "key_value",
    template: {
      key_title: "Key",
      key_description: null,
      key_default: null,
      key_schema: {
        type: "string",
        title: "Key",
      },
      value_title: "Value",
      value_description: null,
      value_default: null,
      value_schema: {
        type: "string",
        format: "ip",
      },
      value_kind: {
        type: "field",
        scalar: "string",
        enum_options: null,
        enum_values: null,
      },
      entry_schema: {
        type: "object",
        properties: {
          key: { type: "string", title: "Key" },
          value: { type: "string", format: "ip", title: "Value" },
        },
        required: ["key"],
      },
    },
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
    expect(screen.getByText("{ id: entry-1, value: 0 }")).toBeTruthy();
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

  it("keeps single composite selector and content in sync for nested object variants", async () => {
    const user = userEvent.setup();
    render(
      <EditorHarness
        node={complexCompositeConfigNode}
        initialValue={[]}
      />,
    );

    const listRadio = screen.getByRole("radio", { name: /list config/i });
    const objectRadio = screen.getByRole("radio", { name: /object config/i });
    expect(listRadio).toHaveAttribute("aria-checked", "true");
    expect(objectRadio).toHaveAttribute("aria-checked", "false");

    await user.click(objectRadio);

    expect(objectRadio).toHaveAttribute("aria-checked", "true");
    expect(listRadio).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("Object Config content:")).toBeTruthy();
    expect(screen.queryByText("List Config content:")).toBeNull();
    expect(screen.getByText("/settings")).toBeTruthy();
    expect(JSON.parse(screen.getByTestId("value").textContent ?? "null")).toEqual(
      {
        mode: "basic",
        settings: {},
      },
    );
  });

  it("materializes object defaults for $defs-backed object-or-null variants", async () => {
    const user = userEvent.setup();
    render(<EditorHarness node={speedtestCompositeNode} initialValue={null} />);

    const objectRadio = screen.getByRole("radio", { name: /objectObject/i });
    const nullRadio = screen.getByRole("radio", { name: /enum\(1\)Null/i });

    expect(nullRadio).toHaveAttribute("aria-checked", "true");
    expect(objectRadio).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("Null content:")).toBeTruthy();
    expect(screen.getByRole("combobox")).toHaveTextContent("null");
    expect(screen.queryByText("Select an option")).toBeNull();

    await user.click(objectRadio);

    expect(objectRadio).toHaveAttribute("aria-checked", "true");
    expect(nullRadio).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("Object content:")).toBeTruthy();
    expect(screen.getByDisplayValue("127.0.0.1:8080")).toBeTruthy();
    expect(screen.getByDisplayValue("5")).toBeTruthy();
    expect(screen.queryByText("Select an option")).toBeNull();
    expect(JSON.parse(screen.getByTestId("value").textContent ?? "null")).toEqual(
      {
        "speedtest-target": "127.0.0.1:8080",
        "expected-rate-mbps": 5,
      },
    );

    await user.click(nullRadio);

    expect(nullRadio).toHaveAttribute("aria-checked", "true");
    expect(objectRadio).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("Null content:")).toBeTruthy();
    expect(screen.getByRole("combobox")).toHaveTextContent("null");
    expect(screen.queryByText("Select an option")).toBeNull();
    expect(JSON.parse(screen.getByTestId("value").textContent ?? "null")).toBeNull();
  });

  it("renders key-value nodes with add-entry controls and saves dns hosts entries", async () => {
    const user = userEvent.setup();
    render(<EditorHarness node={dnsHostsNode} initialValue={{}} />);

    expect(screen.getByText("www.google.com: 8.8.8.8")).toBeTruthy();
    expect(screen.getByRole("button", { name: /\+ add entry/i })).toBeTruthy();

    await user.click(screen.getByRole("button", { name: /\+ add entry/i }));

    const dialog = await screen.findByRole("dialog");
    const textboxes = within(dialog).getAllByRole("textbox");
    await user.type(textboxes[0], "www.google.com");
    await user.type(textboxes[1], "8.8.8.8");
    await user.click(within(dialog).getByRole("button", { name: /^done$/i }));

    expect(screen.getByText("www.google.com")).toBeTruthy();
    expect(screen.getByText("8.8.8.8")).toBeTruthy();
    expect(JSON.parse(screen.getByTestId("value").textContent ?? "null")).toEqual(
      { "www.google.com": "8.8.8.8" },
    );
  });

  it("adds inner padding to overlay scroll region so focused inputs are not clipped", async () => {
    const user = userEvent.setup();
    render(<EditorHarness node={dnsHostsNode} initialValue={{}} />);

    await user.click(screen.getByRole("button", { name: /\+ add entry/i }));

    const dialog = await screen.findByRole("dialog");
    const scrollRegion = within(dialog)
      .getByText("Key")
      .closest("div.max-h-\\[60vh\\]");

    expect(scrollRegion).toBeTruthy();
    expect(scrollRegion?.className).toContain("px-1");
    expect(scrollRegion?.className).toContain("py-1");
    expect(scrollRegion?.className).toContain("sm:px-2");
  });
});
