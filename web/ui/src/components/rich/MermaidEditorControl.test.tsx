import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MermaidEditorControl } from "./MermaidEditorControl";
import { EditorRuntimeProvider } from "./EditorRuntime";
import { ThemeProvider } from "../../theme";
import type { RenderContentInput, RenderContentResult, SchemaUiTransport } from "../../transport/types";

const SVG = { body: "<svg viewBox='0 0 10 10'></svg>", width: 10, height: 10 };

function fakeTransport(
  render: (input: RenderContentInput) => Promise<RenderContentResult>,
): SchemaUiTransport {
  return {
    kind: "http",
    bootstrap: vi.fn(),
    validate: vi.fn(),
    preview: vi.fn(),
    save: vi.fn(),
    exit: vi.fn(),
    renderContent: render,
  };
}

function Editor({
  transport,
  value,
  onChange,
  capabilities,
}: {
  transport: SchemaUiTransport;
  value: string;
  onChange: (next: string) => void;
  capabilities?: import("../../types").Capability[];
}) {
  return (
    <ThemeProvider>
      <EditorRuntimeProvider transport={transport} capabilities={capabilities}>
        <MermaidEditorControl value={value} onChange={onChange} />
      </EditorRuntimeProvider>
    </ThemeProvider>
  );
}

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
});
afterEach(() => {
  vi.useRealTimers();
});

describe("MermaidEditorControl", () => {
  it("is a plain text area when the session offers no renderer", () => {
    render(<Editor transport={fakeTransport(vi.fn())} value="flowchart LR; A-->B" onChange={() => {}} capabilities={[]} />);
    expect(screen.getByRole("textbox", { name: "Mermaid source" })).toBeTruthy();
    // The pane that would render is simply not there — no spinner, no error.
    expect(screen.queryByText(/Rendering|rendered/)).toBeNull();
  });

  it("renders the debounced source and shows the svg", async () => {
    const renderContent = vi.fn().mockResolvedValue({ svg: SVG });
    const transport = fakeTransport(renderContent);
    const { rerender } = render(
      <Editor transport={transport} value="" onChange={() => {}} capabilities={["content_render"]} />,
    );

    rerender(
      <Editor
        transport={transport}
        value="flowchart LR; A-->B"
        onChange={() => {}}
        capabilities={["content_render"]}
      />,
    );
    // Debounced: not called immediately…
    expect(renderContent).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(350);
    expect(renderContent).toHaveBeenCalledWith({
      kind: "mermaid",
      source: "flowchart LR; A-->B",
      theme: expect.any(String),
    });
    await waitFor(() => expect(document.querySelector("svg")).toBeTruthy());
  });

  it("shows the renderer's message when the source does not parse", async () => {
    const renderContent = vi.fn().mockResolvedValue({
      error: { message: "unexpected token at 1:1" },
    });
    render(
      <Editor
        transport={fakeTransport(renderContent)}
        value="flowchart LR; A-->"
        onChange={() => {}}
        capabilities={["content_render"]}
      />,
    );
    await vi.advanceTimersByTimeAsync(350);
    await waitFor(() => expect(screen.getByText(/unexpected token/)).toBeTruthy());
  });

  it("keeps the last good diagram on screen while re-rendering and on errors", async () => {
    const renderContent = vi.fn()
      .mockResolvedValueOnce({ svg: SVG })
      .mockResolvedValue({ error: { message: "unexpected token at 1:1" } });
    const { rerender } = render(
      <Editor
        transport={fakeTransport(renderContent)}
        value="flowchart LR; A-->B"
        onChange={() => {}}
        capabilities={["content_render"]}
      />,
    );
    await vi.advanceTimersByTimeAsync(400);
    await waitFor(() => expect(document.querySelector("svg")).toBeTruthy());

    // The author keeps typing: the pane must hold the previous diagram while
    // the new source renders — a blanking pane reads as a layout collapse.
    rerender(
      <Editor
        transport={fakeTransport(renderContent)}
        value="flowchart LR; A-->B -->"
        onChange={() => {}}
        capabilities={["content_render"]}
      />,
    );
    expect(document.querySelector("svg")).toBeTruthy();
    await vi.advanceTimersByTimeAsync(400);
    // Error arrived: the diagram is still there, with the message along it.
    expect(document.querySelector("svg")).toBeTruthy();
    await act(async () => {});
    expect(screen.getByText(/unexpected token/)).toBeTruthy();
  });

  it("keeps the source editable and reports changes up", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const onChange = vi.fn();
    render(
      <Editor
        transport={fakeTransport(vi.fn())}
        value=""
        onChange={onChange}
        capabilities={[]}
      />,
    );
    await user.type(screen.getByRole("textbox", { name: "Mermaid source" }), "x");
    expect(onChange).toHaveBeenCalledWith("x");
  });
});
