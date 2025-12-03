import Prism from "prismjs";
import "prismjs/components/prism-json";
import "prismjs/components/prism-yaml";
import "prismjs/components/prism-toml";
// Custom theme will be defined in index.css

const LANGUAGE_MAP: Record<string, Prism.Grammar> = {
  json: Prism.languages.json,
  yaml: Prism.languages.yaml,
  toml: Prism.languages.toml,
};

export function normalizedLanguage(format: string): "json" | "yaml" | "toml" {
  if (format === "yaml") return "yaml";
  if (format === "toml") return "toml";
  return "json";
}

export function highlightSyntax(payload: string, format: string): string {
  const languageId = normalizedLanguage(format);
  const grammar = LANGUAGE_MAP[languageId];
  try {
    return Prism.highlight(payload ?? "", grammar, languageId);
  } catch {
    const escape = payload
      ?.replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
    return escape ?? "";
  }
}
