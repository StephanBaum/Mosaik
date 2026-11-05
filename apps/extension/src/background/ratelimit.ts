/** A sliding-window limiter: at most `limit` hits per `windowMs` for each key. */
export class RateLimiter {
  readonly #limit: number;
  readonly #windowMs: number;
  readonly #now: () => number;
  readonly #hits = new Map<string, number[]>();

  constructor(limit: number, windowMs: number, now: () => number = Date.now) {
    this.#limit = limit;
    this.#windowMs = windowMs;
    this.#now = now;
  }

  /** Records a hit and returns whether it is within the limit. */
  allow(key: string): boolean {
    const now = this.#now();
    const recent = (this.#hits.get(key) ?? []).filter((at) => now - at < this.#windowMs);
    if (recent.length >= this.#limit) {
      this.#hits.set(key, recent);
      return false;
    }
    recent.push(now);
    this.#hits.set(key, recent);
    return true;
  }

  forget(key: string): void {
    this.#hits.delete(key);
  }
}
