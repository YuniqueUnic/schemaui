import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import type { SessionResponse } from "./types";
import { ThemeProvider } from "./theme";

const apiMocks = vi.hoisted(() => ({
  persistData: vi.fn(),
  validateData: vi.fn(),
  renderPreview: vi.fn(),
  fetchSession: vi.fn(),
  exitSession: vi.fn(),
}));

vi.mock("./api", () => ({
  persistData: apiMocks.persistData,
  validateData: apiMocks.validateData,
  renderPreview: apiMocks.renderPreview,
  fetchSession: apiMocks.fetchSession,
  exitSession: apiMocks.exitSession,
}));

const toast = vi.hoisted(() => ({
  error: vi.fn(),
  info: vi.fn(),
  success: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast,
}));

const session: SessionResponse = {
  title: "Complex Composite Session",
  description: "Schema-level description should appear in the header.",
  data: {},
  formats: ["json"],
  ui_ast: {
    roots: [
      {
        pointer: "/complexComposite",
        title: "10. Wrong Root Title",
        description: "Wrong root description",
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
                      title: "List Config",
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
            },
          ],
          required: [],
        },
      },
    ],
  },
  layout: null,
};

describe("App web interactions", () => {
  beforeEach(() => {
    localStorage.clear();
    apiMocks.persistData.mockReset();
    apiMocks.validateData.mockReset();
    apiMocks.renderPreview.mockReset();
    apiMocks.fetchSession.mockReset();
    apiMocks.exitSession.mockReset();
    toast.error.mockReset();
    toast.info.mockReset();
    toast.success.mockReset();
    toast.warning.mockReset();

    apiMocks.fetchSession.mockResolvedValue(session);
    apiMocks.validateData.mockResolvedValue({ errors: [] });
    apiMocks.renderPreview.mockResolvedValue({ payload: "{}" });
    apiMocks.persistData.mockResolvedValue({});
    apiMocks.exitSession.mockResolvedValue({});
  });

  it("keeps composite radio selection aligned with rendered content after switching variants", async () => {
    const user = userEvent.setup();
    render(
      <ThemeProvider>
        <App />
      </ThemeProvider>,
    );

    const listRadio = await screen.findByRole("radio", {
      name: /list config/i,
    });
    const objectRadio = screen.getByRole("radio", {
      name: /object config/i,
    });

    expect(listRadio).toHaveAttribute("aria-checked", "true");
    expect(objectRadio).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("List Config content:")).toBeTruthy();

    await user.click(objectRadio);

    expect(objectRadio).toHaveAttribute("aria-checked", "true");
    expect(listRadio).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("Object Config content:")).toBeTruthy();
    expect(screen.queryByText("List Config content:")).toBeNull();
    expect(screen.getByText("/settings")).toBeTruthy();
  });

  it("uses session-level title and description in the page header", async () => {
    render(
      <ThemeProvider>
        <App />
      </ThemeProvider>,
    );

    expect(
      await screen.findByRole("heading", { name: "Complex Composite Session" }),
    ).toBeTruthy();
    expect(
      screen.getByText("Schema-level description should appear in the header."),
    ).toBeTruthy();
  });

  it("shows the selected section description above child fields", async () => {
    apiMocks.fetchSession.mockResolvedValue({
      title: "My title",
      description: "My description",
      data: {},
      formats: ["json"],
      layout: null,
      ui_ast: {
        roots: [
          {
            pointer: "/Foobar",
            title: "Foobar Title",
            description: "Foobar description",
            required: false,
            default_value: {},
            kind: {
              type: "object",
              required: [],
              children: [
                {
                  pointer: "/Foobar/comment",
                  title: "Comment title",
                  description: "Comment description",
                  required: false,
                  default_value: "",
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
      },
    } satisfies SessionResponse);

    render(
      <ThemeProvider>
        <App />
      </ThemeProvider>,
    );

    expect(await screen.findByText("Foobar description")).toBeTruthy();
    expect(screen.getByText("Comment description")).toBeTruthy();
    expect(screen.getAllByText("Comment title").length).toBeGreaterThan(0);
  });
});
