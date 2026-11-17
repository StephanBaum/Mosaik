import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RIPPLE } from "./effects.ts";
import { useHoloLinger } from "./useHoloLinger.ts";

const RIPPLE_MS = Math.ceil(RIPPLE.duration * 1000);

function setReduced(reduced: boolean): void {
  vi.stubGlobal("matchMedia", (query: string) => ({ matches: reduced, media: query }));
}

describe("useHoloLinger", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    setReduced(false);
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("keeps the foil for one ripple after a click hands the primary role away", () => {
    const { result, rerender } = renderHook(({ primary }) => useHoloLinger(primary), {
      initialProps: { primary: true },
    });
    act(() => {
      result.current.pressed();
    });
    rerender({ primary: false });
    expect(result.current.holo).toBe(true);
    act(() => {
      vi.advanceTimersByTime(RIPPLE_MS - 1);
    });
    expect(result.current.holo).toBe(true);
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current.holo).toBe(false);
  });

  it("drops the foil at once when the role changes without a click (shortcut, navigation)", () => {
    const { result, rerender } = renderHook(({ primary }) => useHoloLinger(primary), {
      initialProps: { primary: true },
    });
    rerender({ primary: false });
    expect(result.current.holo).toBe(false);
  });

  it("does not linger under reduced motion (there is no ripple)", () => {
    setReduced(true);
    const { result, rerender } = renderHook(({ primary }) => useHoloLinger(primary), {
      initialProps: { primary: true },
    });
    act(() => {
      result.current.pressed();
    });
    rerender({ primary: false });
    expect(result.current.holo).toBe(false);
  });

  it("does not turn a plain button holo when it is clicked", () => {
    const { result } = renderHook(() => useHoloLinger(false));
    act(() => {
      result.current.pressed();
    });
    expect(result.current.holo).toBe(false);
  });
});
