/** The transition veil, shared between the lazy field chunk (creates it) and the entry's nav wiring (uses it). */
export interface VeilApi {
  wipeIn(seconds: number): Promise<void>;
  reset(): void;
}

let current: VeilApi | null = null;

export function setVeil(v: VeilApi | null): void {
  current = v;
}

export function getVeil(): VeilApi | null {
  return current;
}
