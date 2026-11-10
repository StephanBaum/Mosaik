/**
 * The clock and the Allow guard of the approval prompts (SSH, browser pairing, browser save):
 * Allow stays disabled for 1 s after the prompt opens and after the window regains focus, so a
 * keystroke meant for another window cannot approve anything.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { useWindowFocused } from "./useWindowFocused.ts";

export const ALLOW_GUARD_MS = 1000;
const TICK_MS = 250;

export function useAllowGuard(): { now: number; locked: boolean; restart: () => void } {
  const focused = useWindowFocused();
  const [now, setNow] = useState(() => Date.now());
  const [guardUntil, setGuardUntil] = useState(() => Date.now() + ALLOW_GUARD_MS);
  const wasFocused = useRef(focused);

  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, TICK_MS);
    return () => {
      clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    if (focused && !wasFocused.current) {
      setGuardUntil(Date.now() + ALLOW_GUARD_MS);
    }
    wasFocused.current = focused;
  }, [focused]);

  /** Starts the second again (the prompt shows something new). */
  const restart = useCallback(() => {
    setGuardUntil(Date.now() + ALLOW_GUARD_MS);
  }, []);

  return { now, locked: !focused || now < guardUntil, restart };
}
