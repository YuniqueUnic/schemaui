import Prism from 'prismjs';
import 'prismjs/components/prism-json';
import 'prismjs/components/prism-yaml';
import 'prismjs/components/prism-toml';

const LANGUAGE_MAP: Record<string, Prism.Grammar> = {
  json: Prism.languages.json,
  yaml: Prism.languages.yaml,
  toml: Prism.languages.toml,
};

export function highlightSyntax(payload: string, format: string): string {
  const grammar = LANGUAGE_MAP[format] ?? Prism.languages.json;
  const languageId = grammar === Prism.languages.yaml ? 'yaml'
    : grammar === Prism.languages.toml
      ? 'toml'
      : 'json';
  try {
    return Prism.highlight(payload ?? '', grammar, languageId);
  } catch {
    const escape = payload
      ?.replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
    return escape ?? '';
  }
}
