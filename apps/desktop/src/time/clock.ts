/**
 * One shared clock as an external store, so components read time through
 * `useSyncExternalStore` instead of calling `Date.now()` in render.
 */
import { useSyncExternalStore } from "react";

export interface Clock {
  subscribe: (cb: () => void) => () => void;
  getSnapshot: () => number;
}

export function createClock(tickMs = 1000, now: () => number = Date.now): Clock {
  const listeners = new Set<() => void>();
  let current = now();
  let timeout: ReturnType<typeof setTimeout> | undefined;
  let interval: ReturnType<typeof setInterval> | undefined;

  const tick = (): void => {
    current = now();
    listeners.forEach((cb) => {
      cb();
    });
  };

  const start = (): void => {
    current = now();
    timeout = setTimeout(() => {
      tick();
      interval = setInterval(tick, tickMs);
    }, tickMs - (current % tickMs));
  };

  const stop = (): void => {
    clearTimeout(timeout);
    clearInterval(interval);
    timeout = undefined;
    interval = undefined;
  };

  return {
    subscribe(cb) {
      listeners.add(cb);
      if (listeners.size === 1) {
        start();
      }
      return () => {
        listeners.delete(cb);
        if (listeners.size === 0) {
          stop();
        }
      };
    },
    getSnapshot: () => current,
  };
}

export const appClock: Clock = createClock();

export function useNow(): number {
  return useSyncExternalStore(appClock.subscribe, appClock.getSnapshot);
}
