import { power1Out, type Ease } from "./ease.ts";
import { ticker, type Ticker } from "./ticker.ts";

export interface TweenOptions {
  from: number;
  to: number;
  duration: number;
  delay?: number;
  ease?: Ease;
  onUpdate: (value: number) => void;
}

/** A single number tween. `done` resolves true on completion and false when cancelled. */
export function tween(o: TweenOptions, t: Ticker = ticker): { cancel(): void; done: Promise<boolean> } {
  const ease = o.ease ?? power1Out;
  let elapsed = -(o.delay ?? 0);
  let finished = false;
  let settle: (completed: boolean) => void = () => undefined;
  const done = new Promise<boolean>((resolve) => {
    settle = resolve;
  });
  const stop = t.add((_now, dt) => {
    elapsed += dt;
    if (elapsed < 0) {
      return;
    }
    const p = o.duration <= 0 ? 1 : Math.min(elapsed / o.duration, 1);
    o.onUpdate(o.from + (o.to - o.from) * ease(p));
    if (p >= 1) {
      finished = true;
      stop();
      settle(true);
    }
  });
  return {
    cancel() {
      if (!finished) {
        finished = true;
        stop();
        settle(false);
      }
    },
    done,
  };
}
