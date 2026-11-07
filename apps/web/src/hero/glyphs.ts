/**
 * The five item glyphs etched into one tile in five (key, card, note, Wi-Fi, terminal). The
 * geometry is the app's icon table (apps/desktop/src/components/icons/paths.ts, 24 x 24,
 * Lucide style), as SVG path data so the static page and the WebGL atlas draw the same marks.
 */
export const GLYPHS = {
  key: [
    "M13 15.5a5.5 5.5 0 1 1-11 0a5.5 5.5 0 1 1 11 0",
    "m21 2-9.6 9.6",
    "m15.5 7.5 3 3L22 7l-3-3",
  ],
  card: ["M4 5h16a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Z", "M2 10h20"],
  note: [
    "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z",
    "M14 2v4a2 2 0 0 0 2 2h4",
    "M16 13H8",
    "M16 17H8",
  ],
  wifi: [
    "M12 20h.01",
    "M2 8.82a15 15 0 0 1 20 0",
    "M5 12.859a10 10 0 0 1 14 0",
    "M8.5 16.429a5 5 0 0 1 7 0",
  ],
  terminal: ["m4 17 6-6-6-6", "M12 19h8"],
} as const;

export type GlyphName = keyof typeof GLYPHS;

/** Atlas order; the shader picks `floor(hash * 5)`. */
export const GLYPH_ORDER: readonly GlyphName[] = ["key", "card", "note", "wifi", "terminal"];

/** Stroke width of the app's icons in the 24 px drawing. */
export const GLYPH_STROKE = 1.75;
