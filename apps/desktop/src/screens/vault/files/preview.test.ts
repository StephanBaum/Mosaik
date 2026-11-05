import { describe, expect, it, vi } from "vitest";
import { paintPreview, parsePreview, PREVIEW_HEADER_LEN, wipeBuffer, wipeCanvas } from "./preview.ts";

function answer(width: number, height: number, fill = 7): ArrayBuffer {
  const buffer = new ArrayBuffer(PREVIEW_HEADER_LEN + width * height * 4);
  const view = new DataView(buffer);
  view.setUint32(0, width, true);
  view.setUint32(4, height, true);
  new Uint8Array(buffer, PREVIEW_HEADER_LEN).fill(fill);
  return buffer;
}

describe("parsePreview", () => {
  it("reads the little-endian header and views the pixels in place", () => {
    const buffer = answer(3, 2);
    const parsed = parsePreview(buffer);
    expect([parsed.width, parsed.height]).toEqual([3, 2]);
    expect(parsed.pixels.length).toBe(3 * 2 * 4);
    expect(parsed.pixels.buffer).toBe(buffer);
    expect(parsed.pixels[0]).toBe(7);
  });

  it("refuses a wrong length, an empty side and a short buffer", () => {
    const long = new Uint8Array(PREVIEW_HEADER_LEN + 3 * 2 * 4 + 1);
    long.set(new Uint8Array(answer(3, 2)));
    expect(() => parsePreview(long.buffer)).toThrow();
    expect(() => parsePreview(answer(3, 2).slice(0, -1))).toThrow();
    expect(() => parsePreview(answer(0, 2))).toThrow();
    expect(() => parsePreview(answer(2, 0))).toThrow();
    expect(() => parsePreview(new ArrayBuffer(4))).toThrow();
    // A header that claims more than the buffer holds (2^32 - 1 square) must not pass.
    const huge = new ArrayBuffer(PREVIEW_HEADER_LEN + 16);
    new DataView(huge).setUint32(0, 0xffff_ffff, true);
    new DataView(huge).setUint32(4, 0xffff_ffff, true);
    expect(() => parsePreview(huge)).toThrow();
  });
});

describe("wiping", () => {
  it("wipeBuffer zeroes the buffer, header included", () => {
    const buffer = answer(2, 2, 200);
    const view = parsePreview(buffer).pixels;
    wipeBuffer(buffer);
    expect(new Uint8Array(buffer).every((byte) => byte === 0)).toBe(true);
    expect(view.every((byte) => byte === 0)).toBe(true);
  });

  it("wipeCanvas clears the canvas and sets its size to 0", () => {
    const canvas = document.createElement("canvas");
    canvas.width = 20;
    canvas.height = 10;
    const clearRect = vi.fn();
    vi.spyOn(canvas, "getContext").mockReturnValue({ clearRect } as unknown as CanvasRenderingContext2D);
    wipeCanvas(canvas);
    expect(clearRect).toHaveBeenCalledWith(0, 0, 20, 10);
    expect([canvas.width, canvas.height]).toEqual([0, 0]);
    expect(() => {
      wipeCanvas(null);
    }).not.toThrow();
  });
});

describe("paintPreview", () => {
  it("sizes the canvas, then puts the pixels", () => {
    class FakeImageData {
      constructor(
        readonly data: Uint8ClampedArray,
        readonly width: number,
        readonly height: number,
      ) {}
    }
    vi.stubGlobal("ImageData", FakeImageData);
    const canvas = document.createElement("canvas");
    const putImageData = vi.fn();
    vi.spyOn(canvas, "getContext").mockReturnValue({ putImageData } as unknown as CanvasRenderingContext2D);
    paintPreview(canvas, parsePreview(answer(4, 3)));
    expect([canvas.width, canvas.height]).toEqual([4, 3]);
    expect(putImageData).toHaveBeenCalledOnce();
    const image = putImageData.mock.calls[0]?.[0] as FakeImageData;
    expect([image.width, image.height, image.data.length]).toEqual([4, 3, 48]);
    vi.unstubAllGlobals();
  });

  it("throws without a 2d context", () => {
    const canvas = document.createElement("canvas");
    vi.spyOn(canvas, "getContext").mockReturnValue(null);
    expect(() => {
      paintPreview(canvas, parsePreview(answer(1, 1)));
    }).toThrow();
  });
});
