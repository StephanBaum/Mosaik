import type { Ticker } from "./ticker.ts";

/** A ticker the test steps by hand. */
export function manualTicker(): Ticker & { step(dt: number): void } {
  const subs = new Set<(now: number, dt: number) => void>();
  let now = 0;
  return {
    add(fn) {
      subs.add(fn);
      return () => {
        subs.delete(fn);
      };
    },
    step(dt) {
      now += dt * 1000;
      for (const fn of [...subs]) {
        fn(now, dt);
      }
    },
  };
}
