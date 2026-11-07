/**
 * Scramble-decode type: each character starts as a pixel block, cycles through random glyphs
 * and settles into the real letter, staggered per character. The real letter keeps its place
 * in the layout the whole time (only its paint is hidden and a glyph is drawn over it), so
 * the line never changes width, weight or wrapping.
 */

export const GLYPHS = "ABCDEFGHJKLMNPQRSTUVWXYZ0123456789#$%&*+/<=>?";

export interface ScrambleTiming {
  /** ms before the first character moves. */
  delay: number;
  /** ms between neighbouring characters. */
  stagger: number;
  /** ms a character cycles glyphs before it settles. */
  cycle: number;
  /** ms per glyph change. */
  tick: number;
  /** Characters show as pixel blocks before they start (the load decode) or stay plain (hover). */
  blocks: boolean;
}

export type CharState = { kind: "block"; tone: number } | { kind: "glyph"; glyph: string } | { kind: "plain" };

/** A small integer hash, stable across runs. */
export function hash(n: number): number {
  let x = Math.imul(n ^ 0x9e3779b9, 0x85ebca6b);
  x = Math.imul(x ^ (x >>> 13), 0xc2b2ae35);
  return ((x ^ (x >>> 16)) >>> 0) / 4294967296;
}

/** What character `index` shows `elapsed` ms into the decode. */
export function charState(index: number, elapsed: number, timing: ScrambleTiming): CharState {
  const start = timing.delay + index * timing.stagger;
  if (elapsed < start) {
    return timing.blocks ? { kind: "block", tone: 1 + Math.floor(hash(index * 7 + 3) * 5) } : { kind: "plain" };
  }
  if (elapsed >= start + timing.cycle) {
    return { kind: "plain" };
  }
  const step = Math.floor((elapsed - start) / timing.tick);
  const glyph = GLYPHS[Math.floor(hash(index * 131 + step * 17) * GLYPHS.length)] ?? "#";
  return { kind: "glyph", glyph };
}

/** When the last character settles. */
export function decodeEnd(count: number, timing: ScrambleTiming): number {
  return timing.delay + Math.max(0, count - 1) * timing.stagger + timing.cycle;
}

/** Wraps every non-space character under `el` in a `.ch` span; returns them in reading order. */
export function splitChars(el: Element): HTMLSpanElement[] {
  const spans: HTMLSpanElement[] = [];
  const walker = el.ownerDocument.createTreeWalker(el, NodeFilter.SHOW_TEXT);
  const texts: Text[] = [];
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    texts.push(node as Text);
  }
  for (const text of texts) {
    const fragment = el.ownerDocument.createDocumentFragment();
    for (const char of Array.from(text.data)) {
      if (/\s/.test(char)) {
        fragment.append(char);
        continue;
      }
      const span = el.ownerDocument.createElement("span");
      span.className = "ch";
      span.textContent = char;
      spans.push(span);
      fragment.append(span);
    }
    text.replaceWith(fragment);
  }
  return spans;
}

const STATE_CLASSES = ["is-blk", "is-glyph", "tone-1", "tone-2", "tone-3", "tone-4", "tone-5"];

function apply(span: HTMLSpanElement, state: CharState): void {
  span.classList.remove(...STATE_CLASSES);
  if (state.kind === "block") {
    span.classList.add("is-blk", `tone-${String(state.tone)}`);
  } else if (state.kind === "glyph") {
    span.classList.add("is-glyph");
    span.dataset["g"] = state.glyph;
  }
}

const key = (state: CharState): string =>
  state.kind === "block" ? `b${String(state.tone)}` : state.kind === "glyph" ? state.glyph : "";

/** Shows the decode's first frame (the pixel blocks) until `decode` starts. */
export function prime(spans: readonly HTMLSpanElement[], timing: ScrambleTiming): void {
  spans.forEach((span, index) => {
    apply(span, charState(index, 0, timing));
  });
}

/** Runs the decode over `spans`; resolves when every character has settled. */
export function decode(spans: readonly HTMLSpanElement[], timing: ScrambleTiming): Promise<void> {
  const shown = spans.map(() => "-");
  const end = decodeEnd(spans.length, timing);
  return new Promise((resolve) => {
    const start = performance.now();
    const frame = (now: number): void => {
      const elapsed = now - start;
      spans.forEach((span, index) => {
        const state = charState(index, elapsed, timing);
        if (key(state) !== shown[index]) {
          shown[index] = key(state);
          apply(span, state);
        }
      });
      if (elapsed < end) {
        requestAnimationFrame(frame);
      } else {
        resolve();
      }
    };
    frame(start);
  });
}
