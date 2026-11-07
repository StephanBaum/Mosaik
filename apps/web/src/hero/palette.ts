/**
 * The art palette. The colours live once, in the stylesheet (`--art-0` … `--art-5` in
 * base.css), so the CSS still and the shader share them. The shader blends and snaps in
 * OKLab, where mixing loud colours stays loud instead of going grey.
 */
export const PALETTE_SIZE = 6;

export type Lab = readonly [l: number, a: number, b: number];

function linear(channel: number): number {
  return channel <= 0.04045 ? channel / 12.92 : Math.pow((channel + 0.055) / 1.055, 2.4);
}

/** `#RRGGBB` (or `#RGB`) to OKLab; `null` for anything else. */
export function hexToOklab(hex: string): Lab | null {
  const raw = hex.trim().replace(/^#/, "");
  const full = raw.length === 3 ? raw.replace(/(.)/g, "$1$1") : raw;
  if (!/^[0-9a-f]{6}$/i.test(full)) {
    return null;
  }
  const [r, g, b] = [0, 2, 4].map((i) => linear(Number.parseInt(full.slice(i, i + 2), 16) / 255)) as [number, number, number];
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

/** Fallback when a custom property is missing: ink, purple, cobalt, cyan, lime, pink. */
const FALLBACK = ["#0d0624", "#5b1fd1", "#2340ff", "#9beffa", "#a8f542", "#ff4fcb"] as const;

/** The six palette colours as a flat OKLab array (uniform `vec3[6]`), read from `el`'s style. */
export function readPalette(el: Element): Float32Array {
  const style = getComputedStyle(el);
  const out = new Float32Array(PALETTE_SIZE * 3);
  for (let i = 0; i < PALETTE_SIZE; i++) {
    const lab = hexToOklab(style.getPropertyValue(`--art-${String(i)}`)) ?? hexToOklab(FALLBACK[i] ?? "#000000");
    out.set(lab ?? [0, 0, 0], i * 3);
  }
  return out;
}
