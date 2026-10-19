/**
 * What a failed request means to the person looking at the screen.
 *
 * Two different failures used to surface as the same opaque browser string:
 * the server answered with an error (a body worth showing), and the server
 * never answered at all (`fetch` rejects with "Failed to fetch"). The second
 * one is not a bug in the form — the session process is gone — and the user
 * needs to be told exactly that, because retrying will never help.
 */

/** The server could not be reached: the session has ended, crashed, or moved. */
export class ConnectionLostError extends Error {
  constructor() {
    super(
      "Lost connection to the schemaui server. The session may have been closed or stopped; restart it to continue.",
    );
    this.name = "ConnectionLostError";
  }
}

/**
 * `fetch`, with transport failures named for what they are.
 *
 * Only a rejected `fetch` is rewritten; an HTTP error response is a real
 * answer from a live server and keeps flowing through unchanged.
 */
export async function send(
  input: RequestInfo | URL,
  init?: RequestInit,
): Promise<Response> {
  try {
    return await fetch(input, init);
  } catch (error) {
    if (error instanceof TypeError) throw new ConnectionLostError();
    throw error;
  }
}

/**
 * The text to show for `error`, or `fallback` when it carries none.
 *
 * Server error bodies are `{"error": "..."}`; anything else (plain text from
 * a proxy, an empty body) is shown as it came rather than discarded.
 */
export function errorMessage(error: unknown, fallback: string): string {
  if (!(error instanceof Error) || !error.message.trim()) return fallback;
  try {
    const parsed = JSON.parse(error.message) as { error?: unknown };
    if (typeof parsed.error === "string" && parsed.error.trim()) {
      return parsed.error;
    }
  } catch {
    // Not JSON: the message is already the text to show.
  }
  return error.message;
}
