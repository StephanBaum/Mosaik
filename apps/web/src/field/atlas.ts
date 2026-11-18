import { fieldColors } from "@mosaic/design-tokens";

export const GLYPHS = "0123456789abcdef";

/** 16 hex glyphs in a row (32 x 48 each) for the transition veil's mosaic. */
export function makeAtlas(doc: Document): HTMLCanvasElement {
  const c = doc.createElement("canvas");
  c.width = 512;
  c.height = 48;
  const x = c.getContext("2d");
  if (x === null) {
    return c;
  }
  x.clearRect(0, 0, 512, 48);
  x.fillStyle = `rgb(${fieldColors.label.join(",")})`; // only the alpha channel is read
  x.textAlign = "center";
  x.textBaseline = "middle";
  x.font = "500 38px 'IBM Plex Mono', ui-monospace, 'Cascadia Mono', Menlo, Consolas, monospace";
  for (let i = 0; i < GLYPHS.length; i++) {
    x.fillText(GLYPHS.charAt(i), i * 32 + 16, 25);
  }
  return c;
}
