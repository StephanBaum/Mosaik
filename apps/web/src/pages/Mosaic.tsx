/**
 * The living mosaic's box (design concept §3 M1). It reserves the layout, holds the canvas the
 * hero island paints, and carries the flat fallback: `surface` tiles with item glyphs, shown
 * without JS, without WebGL2 and after a lost context. Decorative: `aria-hidden`; the copy next
 * to it carries the meaning.
 */
import { GLYPH_ORDER } from "../hero/glyphs.ts";

/** Enough tiles for the widest layout (14 columns of 72 px at 2199 px, 9 rows at 2560 px). */
const COLUMNS = 16;
const ROWS = 9;

/** A small integer hash, so the build output is stable. */
function hash(n: number): number {
  let x = Math.imul(n ^ 0x9e3779b9, 0x85ebca6b);
  x = Math.imul(x ^ (x >>> 13), 0xc2b2ae35);
  return ((x ^ (x >>> 16)) >>> 0) / 4294967296;
}

export function Mosaic() {
  const tiles = Array.from({ length: COLUMNS * ROWS }, (_, index) => {
    const glyph = hash(index) < 0.2 ? GLYPH_ORDER[Math.floor(hash(index + 977) * GLYPH_ORDER.length)] : undefined;
    return (
      <span key={index} className="tile">
        {glyph === undefined ? null : (
          <svg className="tile__glyph" viewBox="0 0 24 24" focusable="false">
            <use href={`#glyph-${glyph}`} />
          </svg>
        )}
      </span>
    );
  });
  return (
    <div className="mosaic" aria-hidden="true">
      <div className="mosaic__flat">{tiles}</div>
      <canvas className="mosaic__canvas" />
    </div>
  );
}
