/** Runs `fn` after the page has loaded and the main thread is idle (never before first paint). */
export function afterLoadIdle(fn: () => void, timeoutMs = 3000): void {
  const run = (): void => {
    if ("requestIdleCallback" in window) {
      window.requestIdleCallback(fn, { timeout: timeoutMs });
    } else {
      setTimeout(fn, 200);
    }
  };
  if (document.readyState === "complete") {
    run();
  } else {
    window.addEventListener("load", run, { once: true });
  }
}

/** Resolves when every finite animation running now (the hero's CSS intro) has finished. */
function introsFinished(): Promise<unknown> {
  const running = document.getAnimations().filter((a) => a.effect?.getComputedTiming().iterations !== Number.POSITIVE_INFINITY);
  return Promise.all(running.map((a) => a.finished.catch(() => undefined)));
}

/**
 * Runs `fn` after `load`, once the page's intro animations have finished and no new
 * largest-contentful-paint candidate has appeared for `quietMs` (at most `capMs` after `load`), so
 * lazy chunks never compete with the paint that decides LCP: the hero fades in, and each
 * element counts when it first becomes visible.
 */
export function afterLoadSettled(fn: () => void, quietMs = 300, capMs = 3000): void {
  const settle = (): void => {
    let done = false;
    let timer = 0;
    let observer: PerformanceObserver | undefined;
    const fire = (): void => {
      if (!done) {
        done = true;
        observer?.disconnect();
        clearTimeout(timer);
        fn();
      }
    };
    const arm = (): void => {
      clearTimeout(timer);
      timer = window.setTimeout(fire, quietMs);
    };
    window.setTimeout(fire, capMs);
    void introsFinished().then(() => {
      try {
        observer = new PerformanceObserver(arm);
        observer.observe({ type: "largest-contentful-paint", buffered: true });
      } catch {
        // No LCP entries in this browser: the quiet period alone decides.
      }
      arm();
    });
  };
  if (document.readyState === "complete") {
    settle();
  } else {
    window.addEventListener("load", settle, { once: true });
  }
}
