/** One shared animation-frame loop that runs only while something subscribes to it. */
export interface Ticker {
  /** `dt` is in seconds, clamped to 0.05 so a hidden tab does not fast-forward animations. */
  add(fn: (now: number, dt: number) => void): () => void;
}

export function createTicker(raf: (cb: FrameRequestCallback) => number, caf: (id: number) => void): Ticker {
  const subscribers = new Set<(now: number, dt: number) => void>();
  let handle: number | undefined;
  let last: number | undefined;

  const frame = (now: number): void => {
    const dt = last === undefined ? 0 : Math.min((now - last) / 1000, 0.05);
    last = now;
    for (const fn of [...subscribers]) {
      fn(now, dt);
    }
    handle = subscribers.size > 0 ? raf(frame) : undefined;
  };

  return {
    add(fn) {
      subscribers.add(fn);
      if (handle === undefined) {
        last = undefined;
        handle = raf(frame);
      }
      return () => {
        subscribers.delete(fn);
        if (subscribers.size === 0 && handle !== undefined) {
          caf(handle);
          handle = undefined;
        }
      };
    },
  };
}

let shared: Ticker | undefined;

/** The page's ticker, bound to `window` on first use. */
export const ticker: Ticker = {
  add(fn) {
    shared ??= createTicker(
      (cb) => window.requestAnimationFrame(cb),
      (id) => {
        window.cancelAnimationFrame(id);
      },
    );
    return shared.add(fn);
  },
};
