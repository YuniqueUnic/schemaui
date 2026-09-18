import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  countdownUrgency,
  formatCountdown,
  useCountdown,
} from "./useCountdown";

describe("formatCountdown", () => {
  it("renders whole seconds as zero-padded mm:ss", () => {
    expect(formatCountdown(0)).toBe("00:00");
    expect(formatCountdown(9)).toBe("00:09");
    expect(formatCountdown(60)).toBe("01:00");
    expect(formatCountdown(90)).toBe("01:30");
    expect(formatCountdown(3_599)).toBe("59:59");
  });

  it("keeps minutes uncapped and only widens to hours when needed", () => {
    expect(formatCountdown(3_600)).toBe("1:00:00");
    expect(formatCountdown(5_400)).toBe("1:30:00");
    // 90 minutes stays readable as minutes rather than 1:30:00.
    expect(formatCountdown(5_400 - 3_600)).toBe("30:00");
  });

  it("clamps a negative remainder to zero rather than rendering a negative clock", () => {
    expect(formatCountdown(-5)).toBe("00:00");
  });
});

describe("countdownUrgency", () => {
  it("escalates as the deadline approaches", () => {
    expect(countdownUrgency(600)).toBe("calm");
    expect(countdownUrgency(61)).toBe("calm");
    expect(countdownUrgency(60)).toBe("warning");
    expect(countdownUrgency(11)).toBe("warning");
    expect(countdownUrgency(10)).toBe("critical");
    expect(countdownUrgency(0)).toBe("critical");
  });
});

describe("useCountdown", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("reports no countdown for an unbounded session", () => {
    const { result } = renderHook(() => useCountdown(null));
    expect(result.current).toBeNull();

    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(result.current).toBeNull();
  });

  it("treats an omitted duration as unbounded", () => {
    const { result } = renderHook(() => useCountdown(undefined));
    expect(result.current).toBeNull();
  });

  it("counts down in whole seconds", () => {
    const { result } = renderHook(() => useCountdown(5_000));
    expect(result.current).toBe(5);

    act(() => {
      vi.advanceTimersByTime(1_000);
    });
    expect(result.current).toBe(4);

    act(() => {
      vi.advanceTimersByTime(3_500);
    });
    expect(result.current).toBe(1);
  });

  it("holds at zero once the deadline passes", () => {
    const { result } = renderHook(() => useCountdown(1_000));
    expect(result.current).toBe(1);

    act(() => {
      vi.advanceTimersByTime(1_000);
    });
    expect(result.current).toBe(0);

    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    expect(result.current).toBe(0);
  });

  it("re-anchors when the session hands out a different duration", () => {
    const { result, rerender } = renderHook(
      ({ expiresInMs }: { expiresInMs: number | null }) =>
        useCountdown(expiresInMs),
      { initialProps: { expiresInMs: 30_000 as number | null } },
    );
    expect(result.current).toBe(30);

    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(result.current).toBe(20);

    // A reload mid-session gets the remaining duration, not the original one.
    rerender({ expiresInMs: 5_000 });
    expect(result.current).toBe(5);
  });

  it("stops ticking after unmount", () => {
    const { unmount } = renderHook(() => useCountdown(5_000));
    unmount();

    expect(vi.getTimerCount()).toBe(0);
  });
});
