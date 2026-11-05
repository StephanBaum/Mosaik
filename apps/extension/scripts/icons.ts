/**
 * The Mosaic mark (three black tiles and one accent tile, UX_GUIDELINES §2) as PNGs, written
 * with `node:zlib` so the build needs no image dependency.
 */
import { deflateSync } from "node:zlib";

export const ICON_SIZES = [16, 32, 48, 128] as const;

const INK: Rgba = [9, 9, 11, 255];
const ACCENT: Rgba = [165, 180, 252, 255];
const TILE = 9 / 20;
const RADIUS = 2.5 / 20;

type Rgba = [number, number, number, number];

/** Coverage (0 to 1) of a rounded square at (x, y) in unit coordinates, by 4 x 4 supersampling. */
function coverage(px: number, py: number, size: number, left: number, top: number): number {
  let hits = 0;
  for (let sy = 0; sy < 4; sy++) {
    for (let sx = 0; sx < 4; sx++) {
      const x = (px + (sx + 0.5) / 4) / size - left;
      const y = (py + (sy + 0.5) / 4) / size - top;
      if (x < 0 || y < 0 || x > TILE || y > TILE) {
        continue;
      }
      const dx = Math.max(RADIUS - x, x - (TILE - RADIUS), 0);
      const dy = Math.max(RADIUS - y, y - (TILE - RADIUS), 0);
      if (dx * dx + dy * dy <= RADIUS * RADIUS) {
        hits++;
      }
    }
  }
  return hits / 16;
}

function pixel(px: number, py: number, size: number): Rgba {
  const tiles: [number, number, Rgba][] = [
    [0, 0, INK],
    [11 / 20, 0, INK],
    [0, 11 / 20, INK],
    [11 / 20, 11 / 20, ACCENT],
  ];
  for (const [left, top, color] of tiles) {
    const alpha = coverage(px, py, size, left, top);
    if (alpha > 0) {
      return [color[0], color[1], color[2], Math.round(alpha * 255)];
    }
  }
  return [0, 0, 0, 0];
}

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) {
    c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  }
  return c >>> 0;
});

function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc = (CRC_TABLE[(crc ^ byte) & 0xff] ?? 0) ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type: string, data: Uint8Array): Buffer {
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const out = Buffer.alloc(body.length + 8);
  out.writeUInt32BE(data.length, 0);
  body.copy(out, 4);
  out.writeUInt32BE(crc32(body), body.length + 4);
  return out;
}

/** A `size` x `size` RGBA PNG of the mark. */
export function renderIcon(size: number): Buffer {
  const rows: Buffer[] = [];
  for (let y = 0; y < size; y++) {
    const row = Buffer.alloc(1 + size * 4);
    for (let x = 0; x < size; x++) {
      row.set(pixel(x, y, size), 1 + x * 4);
    }
    rows.push(row);
  }
  const header = Buffer.alloc(13);
  header.writeUInt32BE(size, 0);
  header.writeUInt32BE(size, 4);
  header.set([8, 6, 0, 0, 0], 8);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(Buffer.concat(rows))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}
