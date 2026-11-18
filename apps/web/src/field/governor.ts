/** Frames ignored first: shader compile and first paint are not representative. */
const WARM_UP = 30;
const WINDOW = 90;
const LITE_ABOVE_MS = 25;
const STOP_ABOVE_MS = 45;

export type Verdict = "keep" | "lite" | "stop";

/**
 * Watches the frame interval and degrades in two steps: after the warm-up, a 90-frame mean
 * above 25 ms means "lite" (once); in lite, a mean above 45 ms means "stop" (show the still).
 * `sample` returns the verdict only on the frame that completes a window, "keep" otherwise.
 */
export function createGovernor(): { sample(dtMs: number): Verdict } {
  let seen = 0;
  let sum = 0;
  let count = 0;
  let lite = false;
  return {
    sample(dtMs) {
      seen += 1;
      if (seen <= WARM_UP) {
        return "keep";
      }
      sum += dtMs;
      count += 1;
      if (count < WINDOW) {
        return "keep";
      }
      const mean = sum / count;
      sum = 0;
      count = 0;
      if (lite) {
        return mean > STOP_ABOVE_MS ? "stop" : "keep";
      }
      if (mean > LITE_ABOVE_MS) {
        lite = true;
        return "lite";
      }
      return "keep";
    },
  };
}
