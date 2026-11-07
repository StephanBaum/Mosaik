/**
 * The hero art panel and its decorations, rendered at build time. The panel holds the canvas
 * the hero island paints and the designed CSS still (shown without JS and without WebGL2):
 * the clear gradient with one encrypted mosaic band and a few sorted streaks. The dither
 * blobs are halftone dot fields computed here, so they cost no script. All decorative:
 * `aria-hidden`; the copy carries the meaning.
 */
import { hash } from "../scramble.ts";

const BAND_COLUMNS = 24;
const BAND_ROWS = 3;
const STREAKS = 9;
const PIXELS = 10;

/** Tone 0 is the ink / lavender base, 1 to 5 the loud colours, ordered across the band. */
export function bandTone(column: number, row: number): number {
  const n = row * BAND_COLUMNS + column;
  if (hash(n * 3 + 1) < 0.16) {
    return 0;
  }
  const drift = (hash(n * 5 + 2) - 0.5) * 1.8;
  return 1 + Math.min(4, Math.max(0, Math.floor((column / BAND_COLUMNS) * 5 + drift)));
}

function Still() {
  const cells = [];
  for (let row = 0; row < BAND_ROWS; row++) {
    for (let column = 0; column < BAND_COLUMNS; column++) {
      cells.push(<span key={`${String(row)}-${String(column)}`} className={`art__cell tone-${String(bandTone(column, row))}`} />);
    }
  }
  return (
    <div className="art__still">
      <div className="art__band">
        {cells}
        <div className="art__streaks">
          {Array.from({ length: STREAKS }, (_, i) => (
            <span key={i} className={`art__streak tone-${String(1 + Math.floor(hash(i * 11 + 4) * 5))}`} />
          ))}
        </div>
      </div>
    </div>
  );
}

export function Art() {
  return (
    <div className="art" aria-hidden="true">
      <Still />
      <canvas className="art__canvas" />
    </div>
  );
}

/** One halftone blob: dot radii follow a field of three soft metaballs (deterministic per seed). */
export function ditherPath(seed: number, size: number, pitch: number): string {
  const balls = [0, 1, 2].map((i) => ({
    x: size * (0.25 + 0.5 * hash(seed * 97 + i * 13)),
    y: size * (0.25 + 0.5 * hash(seed * 89 + i * 29)),
    r: size * (0.16 + 0.14 * hash(seed * 71 + i * 7)),
  }));
  const parts: string[] = [];
  const max = pitch * 0.46;
  for (let y = pitch / 2; y < size; y += pitch) {
    for (let x = pitch / 2; x < size; x += pitch) {
      const field = balls.reduce((sum, b) => sum + (b.r * b.r) / ((x - b.x) ** 2 + (y - b.y) ** 2 + 1), 0);
      const r = max * Math.min(1, Math.max(0, field - 0.55)) ** 0.75;
      if (r >= 0.45) {
        const rr = r.toFixed(2);
        parts.push(`M${(x - r).toFixed(2)} ${y.toFixed(2)}a${rr} ${rr} 0 1 0 ${(2 * r).toFixed(2)} 0a${rr} ${rr} 0 1 0 ${(-2 * r).toFixed(2)} 0`);
      }
    }
  }
  return parts.join("");
}

const DITHER_SIZE = 240;
const DITHER_PITCH = 8;

export function Dither({ seed, className }: { seed: number; className: string }) {
  return (
    <svg className={`dither ${className}`} viewBox={`0 0 ${String(DITHER_SIZE)} ${String(DITHER_SIZE)}`} aria-hidden="true" focusable="false">
      <path d={ditherPath(seed, DITHER_SIZE, DITHER_PITCH)} />
    </svg>
  );
}

/** Small squares that rise and fade near the panel edge (CSS animation; still under reduced motion). */
export function Pixels() {
  return (
    <div className="pixels" aria-hidden="true">
      {Array.from({ length: PIXELS }, (_, i) => (
        <span key={i} className={`px tone-${String(1 + (i % 5))}`} />
      ))}
    </div>
  );
}
