/** The glyph atlas of the mosaic: the five item glyphs side by side, rasterised once. */
import type { HoloGl } from "../fx/gl.ts";
import { GLYPH_ORDER, GLYPH_STROKE, GLYPHS } from "./glyphs.ts";

/** Device px per glyph cell in the atlas (mipmapped down for small tiles). */
export const ATLAS_CELL = 128;

function paintAtlas(): HTMLCanvasElement | null {
  const canvas = document.createElement("canvas");
  canvas.width = ATLAS_CELL * GLYPH_ORDER.length;
  canvas.height = ATLAS_CELL;
  const ctx = canvas.getContext("2d");
  if (ctx === null) {
    return null;
  }
  const scale = ATLAS_CELL / 24;
  ctx.strokeStyle = "#000";
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.lineWidth = GLYPH_STROKE * 1.15;
  GLYPH_ORDER.forEach((name, index) => {
    ctx.setTransform(scale, 0, 0, scale, index * ATLAS_CELL, 0);
    for (const d of GLYPHS[name]) {
      ctx.stroke(new Path2D(d));
    }
  });
  return canvas;
}

/** Uploads the atlas to texture unit 0; `null` when 2D canvas is unavailable. */
export function createAtlasTexture(gl: HoloGl): WebGLTexture | null {
  const image = paintAtlas();
  if (image === null) {
    return null;
  }
  const texture = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
  gl.generateMipmap(gl.TEXTURE_2D);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return texture;
}
