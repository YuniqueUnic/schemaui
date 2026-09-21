/**
 * Save `content` as a local file via the browser's normal download flow.
 *
 * Used by the Playground's exit action: there is no server to hand the
 * finished document to, so downloading it is what "finishing" means there.
 */
export function downloadTextFile(
  filename: string,
  content: string,
  mimeType = "text/plain",
): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  // Deferred rather than immediate: revoking synchronously races the
  // download's own read of the blob URL in some browsers.
  setTimeout(() => URL.revokeObjectURL(url), 0);
}
