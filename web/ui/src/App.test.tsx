import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
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
  api_version: "1.0",
  capabilities: [],
  draft_restored: false,
  title: "Complex Composite Session",
  description: "Schema-level description should appear in the header.",
  data: {},
  formats: ["json"],
  expires_in_ms: null,
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

/** A field whose visibility is driven by a sibling boolean. */
const conditionalSession: SessionResponse = {
  api_version: "1.0",
  capabilities: [],
  draft_restored: false,
  title: "Conditional Session",
  description: null,
  data: {},
  formats: ["json"],
  expires_in_ms: null,
  layout: null,
  ui_ast: {
    roots: [
      {
        pointer: "/brief",
        title: "Brief",
        description: null,
        required: false,
        default_value: {},
        kind: {
          type: "object",
          required: [],
          children: [
            {
              pointer: "/brief/advanced",
              title: "Advanced mode",
              description: null,
              required: false,
              default_value: false,
              kind: {
                type: "field",
                scalar: "boolean",
                enum_options: null,
                enum_values: null,
              },
            },
            {
              pointer: "/brief/notes",
              title: "Advanced notes",
              description: null,
              required: false,
              default_value: "",
              visible_when: { field: "advanced", op: "equals", value: true },
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
};

const multilineSession: SessionResponse = {
  api_version: "1.0",
  capabilities: [],
  draft_restored: false,
  title: "Multiline Session",
  description: null,
  data: {},
  formats: ["json"],
  expires_in_ms: null,
  layout: null,
  ui_ast: {
    roots: [
      {
        pointer: "/notes",
        title: "Notes",
        description: null,
        required: false,
        default_value: "line one\nline two",
        kind: {
          type: "field",
          scalar: "string",
          enum_options: null,
          enum_values: null,
          multiline: true,
        },
      },
    ],
  },
};

/**
 * Two root sections, which is the smallest shape that exercises the tab band
 * and the root view. Each section holds one plainly named field so the tests
 * can tell which sections are on screen.
 */
const sectionedSession: SessionResponse = {
  api_version: "1.0",
  capabilities: [],
  draft_restored: false,
  title: "Sectioned Session",
  description: null,
  data: {},
  formats: ["json"],
  expires_in_ms: null,
  layout: null,
  ui_ast: {
    roots: [
      {
        pointer: "/network",
        title: "Network",
        description: null,
        required: false,
        default_value: {},
        kind: {
          type: "object",
          required: [],
          children: [
            {
              pointer: "/network/host",
              title: "Host",
              description: null,
              required: false,
              default_value: "127.0.0.1",
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
      {
        pointer: "/basics",
        title: "Basics",
        description: null,
        required: false,
        default_value: {},
        kind: {
          type: "object",
          required: [],
          children: [
            {
              pointer: "/basics/region",
              title: "Region",
              description: null,
              required: false,
              default_value: "eu",
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
};

/**
 * The same two sections, plus the layout the server always sends.
 *
 * `layout.roots[0]` is the *first section*, not the document, so a root label
 * read from there comes out as "Network" — which is what made the General tab
 * and the tree's root row both announce the wrong name.
 */
const sectionedSessionWithLayout: SessionResponse = {
  ...sectionedSession,
  layout: {
    roots: [
      {
        id: "network",
        title: "Network",
        description: null,
        sections: [
          {
            id: "network",
            title: "Network",
            description: null,
            pointer: "/network",
            path: ["network"],
            field_pointers: ["/network/host"],
            children: [],
          },
        ],
      },
      {
        id: "basics",
        title: "Basics",
        description: null,
        sections: [
          {
            id: "basics",
            title: "Basics",
            description: null,
            pointer: "/basics",
            path: ["basics"],
            field_pointers: ["/basics/region"],
            children: [],
          },
        ],
      },
    ],
  },
};

/** Wait for the section tab band to appear. */
async function waitForSections(): Promise<void> {
  await screen.findByRole("navigation", { name: "Sections" });
}

/**
 * A button in the section tab band.
 *
 * Synchronous on purpose: an async getter invites a missing `await` to be read
 * as a passing assertion, which is exactly what happened while writing these.
 * Call `waitForSections` first.
 */
function sectionTab(name: string): HTMLElement {
  const band = screen.getByRole("navigation", { name: "Sections" });
  return within(band).getByRole("button", { name });
}

/**
 * The editor pane.
 *
 * The tree on the left lists every field by name too, so an unscoped query for
 * "Host" would match both the tree row and the form control.
 */
function editorPane(): HTMLElement {
  const main = document.querySelector("main");
  if (!main) throw new Error("the editor pane should be rendered");
  return main as HTMLElement;
}

/** Whether a field with this label is present in the editor pane. */
function fieldInEditor(label: string): boolean {
  return within(editorPane()).queryAllByText(label).length > 0;
}

function renderApp() {
  return render(
    <ThemeProvider>
      <App />
    </ThemeProvider>,
  );
}
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
    renderApp();

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
    renderApp();

    expect(
      await screen.findByRole("heading", { name: "Complex Composite Session" }),
    ).toBeTruthy();
    expect(
      screen.getByText("Schema-level description should appear in the header."),
    ).toBeTruthy();
  });

  it("shows the selected section description above child fields", async () => {
    apiMocks.fetchSession.mockResolvedValue({
      api_version: "1.0",
      capabilities: [],
      draft_restored: false,
      title: "My title",
      description: "My description",
      data: {},
      formats: ["json"],
      expires_in_ms: null,
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

    renderApp();

    expect(await screen.findByText("Foobar description")).toBeTruthy();
    expect(screen.getByText("Comment description")).toBeTruthy();
    expect(screen.getAllByText("Comment title").length).toBeGreaterThan(0);
  });

  it("hides a conditional field until its controlling sibling matches", async () => {
    apiMocks.fetchSession.mockResolvedValue(conditionalSession);

    const user = userEvent.setup();
    renderApp();

    const toggle = await screen.findByRole("switch");
    // Hidden everywhere: navigation pills, tree and editor all read the same pruned AST.
    expect(screen.queryAllByText("Advanced notes")).toHaveLength(0);

    await user.click(toggle);

    expect(await screen.findAllByText("Advanced notes")).not.toHaveLength(0);
  });

  it("renders a multiline field as a textarea", async () => {
    apiMocks.fetchSession.mockResolvedValue(multilineSession);

    renderApp();

    const control = await screen.findByRole("textbox");
    expect(control.tagName).toBe("TEXTAREA");
    expect(control).toHaveValue("line one\nline two");
  });

  it("names every section in the tab band, General first", async () => {
    apiMocks.fetchSession.mockResolvedValue(sectionedSession);

    renderApp();

    // Scoped to the band: the tree lists the same sections, and matching both
    // would make this assertion about the wrong control.
    const band = await screen.findByRole("navigation", { name: "Sections" });
    const tabs = within(band).getAllByRole("button");
    expect(tabs.map((tab) => tab.textContent)).toEqual([
      "General",
      "Network",
      "Basics",
    ]);
  });

  it("still calls the root tab General when the session carries a layout", async () => {
    apiMocks.fetchSession.mockResolvedValue(sectionedSessionWithLayout);

    renderApp();

    await waitForSections();

    // A layout is always present in a real session, and its first root is the
    // first section. Reading the root label from there named the tab after one
    // of its own children.
    expect(sectionTab("General")).toBeTruthy();
    expect(
      within(screen.getByRole("navigation", { name: "Sections" }))
        .queryAllByRole("button")
        .map((tab) => tab.textContent),
    ).toEqual(["General", "Network", "Basics"]);
  });

  it("opens on General, showing every section at once", async () => {
    apiMocks.fetchSession.mockResolvedValue(sectionedSession);

    renderApp();

    await waitForSections();

    // Both sections' fields, without touching a tab. This is the whole point of
    // the root tab: browsing the form must not require visiting each section.
    expect(fieldInEditor("Host")).toBe(true);
    expect(fieldInEditor("Region")).toBe(true);
  });

  it("marks General as current until a section is picked", async () => {
    apiMocks.fetchSession.mockResolvedValue(sectionedSession);

    renderApp();

    await waitForSections();
    expect(sectionTab("General")).toHaveAttribute("aria-current", "true");
    expect(sectionTab("Network")).not.toHaveAttribute("aria-current");
  });

  it("narrows to one section when its tab is picked", async () => {
    const user = userEvent.setup();
    apiMocks.fetchSession.mockResolvedValue(sectionedSession);

    renderApp();

    await waitForSections();
    await user.click(sectionTab("Network"));

    expect(fieldInEditor("Host")).toBe(true);
    // The other section's field is gone from the form, not merely scrolled
    // past — the tree still lists it, which is why this is scoped.
    expect(fieldInEditor("Region")).toBe(false);
    expect(sectionTab("Network")).toHaveAttribute("aria-current", "true");
  });

  it("returns to the whole document when General is picked again", async () => {
    const user = userEvent.setup();
    apiMocks.fetchSession.mockResolvedValue(sectionedSession);

    renderApp();

    await waitForSections();
    await user.click(sectionTab("Network"));
    expect(fieldInEditor("Region")).toBe(false);

    await user.click(sectionTab("General"));

    // Without this the root view was a one-way door: once a section had been
    // chosen there was no way back to the full form.
    expect(fieldInEditor("Region")).toBe(true);
    expect(fieldInEditor("Host")).toBe(true);
  });

  it("offers the root row in the tree, not only in layout mode", async () => {
    apiMocks.fetchSession.mockResolvedValue(sectionedSession);

    renderApp();

    await waitForSections();

    // Schema mode used to omit the tree row entirely, so the only way to see
    // the whole document was layout mode — which needs a `layout` in the
    // session, and this fixture has none.
    const nav = document.querySelector("aside");
    expect(nav).not.toBeNull();
    expect(
      within(nav as HTMLElement).getByRole("button", { name: "General" }),
    ).toBeTruthy();
  });

  it("hides the tab band when there is only one section", async () => {
    apiMocks.fetchSession.mockResolvedValue(multilineSession);

    renderApp();

    await screen.findByRole("textbox");
    // A lone "General" tab beside a single section would be two names for the
    // same page.
    expect(screen.queryByRole("navigation", { name: "Sections" })).toBeNull();
  });

  it("shows the remaining time when the session carries a deadline", async () => {
    apiMocks.fetchSession.mockResolvedValue({
      ...session,
      expires_in_ms: 90_000,
    });

    renderApp();

    const countdown = await screen.findByRole("timer");
    expect(countdown.textContent).toContain("01:30");
  });

  it("shows no countdown when the session is unbounded", async () => {
    renderApp();

    // Wait for the form itself, so the assertion is about the loaded shell.
    await screen.findByText("Complex Composite Session");
    expect(screen.queryByRole("timer")).toBeNull();
  });

  it("closes the form and says nothing was saved once the deadline passes", async () => {
    apiMocks.fetchSession.mockResolvedValue({ ...session, expires_in_ms: 400 });

    renderApp();

    expect(await screen.findByText("Session Timed Out", {}, { timeout: 4_000 }))
      .toBeTruthy();
    expect(screen.getByTestId("session-notice").textContent)
      .toContain("nothing was saved");
    // The form is gone, so nothing can be typed into a session that is over.
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("explains that the session is gone when it cannot be loaded", async () => {
    apiMocks.fetchSession.mockRejectedValue(new Error("connection refused"));

    renderApp();

    expect(await screen.findByText("Session Unavailable")).toBeTruthy();
  });
});
