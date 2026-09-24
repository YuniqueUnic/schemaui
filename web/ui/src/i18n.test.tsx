import { afterEach, describe, expect, it } from "vitest";
import { act, render, screen } from "@testing-library/react";
import { I18nProvider, useI18n, zhDictionary } from "./i18n";

function Probe({ tKey, params }: { tKey: string; params?: Record<string, string | number> }) {
  const { t, locale, setLocale } = useI18n();
  return (
    <div>
      <span data-testid="out">{t(tKey, params)}</span>
      <span data-testid="locale">{locale}</span>
      <button onClick={() => setLocale(locale === "en" ? "zh" : "en")}>flip</button>
    </div>
  );
}

afterEach(() => {
  localStorage.removeItem("schemaui-locale");
});

describe("i18n", () => {
  it("speaks English outside a provider: the key is the text", () => {
    render(<Probe tKey="Save" />);
    expect(screen.getByTestId("out").textContent).toBe("Save");
  });

  it("interpolates parameters into the template", () => {
    render(<Probe tKey="Errors {count}" params={{ count: 3 }} />);
    expect(screen.getByTestId("out").textContent).toBe("Errors 3");
  });

  it("translates when the locale is zh and persists the choice", () => {
    render(
      <I18nProvider>
        <Probe tKey="Errors {count}" params={{ count: 2 }} />
      </I18nProvider>,
    );
    expect(screen.getByTestId("out").textContent).toBe("Errors 2");
    act(() => {
      screen.getByText("flip").click();
    });
    expect(screen.getByTestId("out").textContent).toBe("2 个错误");
    expect(localStorage.getItem("schemaui-locale")).toBe("zh");
  });

  it("falls back to the English key when a translation is missing", () => {
    localStorage.setItem("schemaui-locale", "zh");
    render(
      <I18nProvider>
        <Probe tKey="A string no one translated" />
      </I18nProvider>,
    );
    expect(screen.getByTestId("out").textContent).toBe("A string no one translated");
  });

  it("honours a stored locale on mount", () => {
    localStorage.setItem("schemaui-locale", "zh");
    render(
      <I18nProvider>
        <Probe tKey="Ready" />
      </I18nProvider>,
    );
    expect(screen.getByTestId("out").textContent).toBe("就绪");
  });

  it("keeps the dictionary honest: every entry is a non-empty string", () => {
    const entries = Object.entries(zhDictionary);
    expect(entries.length).toBeGreaterThan(80);
    for (const [key, value] of entries) {
      expect(typeof value, `non-string translation for "${key}"`).toBe("string");
      expect(value.length, `empty translation for "${key}"`).toBeGreaterThan(0);
    }
  });
});
