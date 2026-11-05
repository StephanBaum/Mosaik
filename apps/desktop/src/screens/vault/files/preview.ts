/**
 * The image preview answer of `attachment_preview` (docs/DESKTOP_IPC.md §6):
 * `[width u32 LE][height u32 LE][width * height * 4 bytes of straight RGBA]` in one
 * `ArrayBuffer`. The pixels are painted into a canvas, and the buffer is wiped right after.
 * Nothing here creates a URL, a blob or an `<img>`: the CSP allows none (ADR-0034).
 */

export const PREVIEW_HEADER_LEN = 8;

export interface ParsedPreview {
  width: number;
  height: number;
  /** A view of the answer's buffer, not a copy: wiping the buffer wipes it. */
  pixels: Uint8ClampedArray<ArrayBuffer>;
}

/**
 * Reads the header and checks the length.
 * @throws when the buffer is not exactly a header and `width * height * 4` bytes, or a side is 0.
 */
export function parsePreview(buffer: ArrayBuffer): ParsedPreview {
  if (buffer.byteLength < PREVIEW_HEADER_LEN) {
    throw new Error("preview: too short");
  }
  const view = new DataView(buffer);
  const width = view.getUint32(0, true);
  const height = view.getUint32(4, true);
  if (width === 0 || height === 0 || buffer.byteLength !== PREVIEW_HEADER_LEN + width * height * 4) {
    throw new Error("preview: wrong length");
  }
  return { width, height, pixels: new Uint8ClampedArray(buffer, PREVIEW_HEADER_LEN, width * height * 4) };
}

/** Paints the pixels at 1:1; CSS scales the canvas down to fit. */
export function paintPreview(canvas: HTMLCanvasElement, parsed: ParsedPreview): void {
  const context = canvas.getContext("2d");
  if (context === null) {
    throw new Error("preview: no 2d context");
  }
  canvas.width = parsed.width;
  canvas.height = parsed.height;
  context.putImageData(new ImageData(parsed.pixels, parsed.width, parsed.height), 0, 0);
}

/** Overwrites the answer's buffer with zeros. */
export function wipeBuffer(buffer: ArrayBuffer): void {
  new Uint8Array(buffer).fill(0);
}

/** Clears the canvas and shrinks it to nothing, so no pixel stays in its backing store. */
export function wipeCanvas(canvas: HTMLCanvasElement | null): void {
  if (canvas === null) {
    return;
  }
  canvas.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
  canvas.width = 0;
  canvas.height = 0;
}
