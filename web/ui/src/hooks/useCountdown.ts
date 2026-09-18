/**
 * Live countdown to a session deadline.
 *
 * The server hands out `expires_in_ms` — a *duration*, not a deadline — so the
 * countdown is anchored to the moment the payload arrived. That keeps it
 * correct on a client whose clock is wrong, which matters because these
 * sessions are routinely opened from a phone or a second machine on the LAN.
 *
 * The client owns the visible countdown rather than polling for it: once the
 * deadline passes the server stops answering, so waiting to be told "you timed
 * out" would leave a dead form on screen until some request happened to fail.
 *
 * Returns the whole seconds left, or `null` for an unbounded session.
 */

import { useEffect, useState } from "react";

/** How often the clock is re-read. Only whole-second changes re-render. */
const POLL_MS = 250;

interface CountdownState {
  /** The duration this countdown was anchored to; drives re-anchoring. */
  duration: number | null;
  secondsLeft: number | null;
}

export function useCountdown(
  expiresInMs: number | null | undefined,
): number | null {
  const duration = expiresInMs ?? null;

  const [state, setState] = useState<CountdownState>(() => ({
    duration,
    secondsLeft: initialSeconds(duration),
  }));

  // Re-anchored during render rather than in an effect. An effect would paint
  // one frame of the previous session's clock before correcting itself, and a
  // reload legitimately hands out a *shorter* remaining duration that must not
  // be shown as the original one.
  if (state.duration !== duration) {
    setState({ duration, secondsLeft: initialSeconds(duration) });
  }

  useEffect(() => {
    if (duration == null) return;

    // The deadline is computed here, once per duration, and never re-derived
    // from the current time — otherwise it would drift forward on every tick
    // and the countdown would never reach zero.
    const deadline = Date.now() + duration;
    const timer = window.setInterval(() => {
      const next = secondsUntil(deadline, Date.now());
      setState((previous) =>
        previous.secondsLeft === next
          // Returning the same object lets React bail out, so polling four
          // times a second still costs exactly one render per displayed second.
          ? previous
          : { duration, secondsLeft: next }
      );
    }, POLL_MS);
    return () => window.clearInterval(timer);
  }, [duration]);

  return state.secondsLeft;
}

/** Seconds to show before the first tick, derived from the duration alone. */
function initialSeconds(duration: number | null): number | null {
  return duration == null ? null : Math.max(0, Math.ceil(duration / 1000));
}

/**
 * Whole seconds between `now` and `deadline`, floored at zero.
 *
 * Ceiling, so the display reads 00:01 through the final second and only reaches
 * 00:00 at the deadline itself.
 */
function secondsUntil(deadline: number, now: number): number {
  return Math.max(0, Math.ceil((deadline - now) / 1000));
}

/**
 * Render whole seconds as `mm:ss`, widening to `h:mm:ss` only when needed.
 *
 * Minutes are not capped at 60 — a 90-minute session reads `90:00`, which is
 * easier to scan than `1:30:00` for the durations this is actually used with.
 */
export function formatCountdown(totalSeconds: number): string {
  const safe = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(safe / 3600);
  const minutes = Math.floor((safe % 3600) / 60);
  const seconds = safe % 60;
  const mm = String(minutes).padStart(2, "0");
  const ss = String(seconds).padStart(2, "0");
  return hours > 0 ? `${hours}:${mm}:${ss}` : `${mm}:${ss}`;
}

/** Urgency band for the countdown, so the badge can change tone as time runs out. */
export type CountdownUrgency = "calm" | "warning" | "critical";

export function countdownUrgency(secondsLeft: number): CountdownUrgency {
  if (secondsLeft <= 10) return "critical";
  if (secondsLeft <= 60) return "warning";
  return "calm";
}
