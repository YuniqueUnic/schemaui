export function highlightSyntax(payload: string, format: string): string {
  const escaped = payload
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  const tokenized = escaped.replace(
    /(\"(?:\\u[0-9a-fA-F]{4}|\\[^u]|[^\\\"])*\"\\s*:?)|(\\b(true|false|null)\\b)|(-?\\d+(?:\\.\\d*)?(?:[eE][+-]?\\d+)?)/g,
    (match, str, bool, _, num) => {
      if (str) {
        return /:$/.test(str)
          ? `<span class="text-cyan-400 dark:text-cyan-300">${str}</span>`
          : `<span class="text-emerald-300 dark:text-emerald-200">${str}</span>`;
      }
      if (bool) {
        return `<span class="text-orange-300 dark:text-orange-200">${bool}</span>`;
      }
      if (num) {
        return `<span class="text-rose-300 dark:text-rose-200">${num}</span>`;
      }
      return match;
    },
  );

  if (format === 'yaml' || format === 'toml') {
    return tokenized.replace(
      /(^|\s)([A-Za-z0-9_-]+)(?=\s*:)/gm,
      (_, prefix, key) =>
        `${prefix}<span class="text-sky-300 dark:text-sky-200">${key}</span>`,
    );
  }

  return tokenized;
}
