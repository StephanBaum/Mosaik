import { describe, expect, it } from "vitest";
import { FRAG, VERT } from "../fx/shader.ts";
import { FIELD_FRAG, FIELD_VERT } from "./shaders.ts";

/** GLSL ES 3.00 keywords reserved for future use, and other words that fail to compile as identifiers. */
const RESERVED = ["attribute","varying","coherent","volatile","restrict","readonly","writeonly","resource","atomic_uint","noperspective","patch","sample","subroutine","common","partition","active","asm","class","union","enum","typedef","template","this","goto","inline","noinline","public","static","extern","external","interface","long","short","double","half","fixed","unsigned","superp","input","output","hvec2","hvec3","hvec4","dvec2","dvec3","dvec4","fvec2","fvec3","fvec4","sampler3DRect","filter","sizeof","cast","namespace","using","image1D","image2D","image3D","imageCube","image1DArray","image2DArray","imageBuffer","iimage1D","iimage2D","iimage3D","uimage1D","uimage2D","uimage3D"];
const SOURCES = { FIELD_VERT, FIELD_FRAG, HOLO_VERT: VERT, HOLO_FRAG: FRAG };

describe("shader sources", () => {
  it.each(Object.entries(SOURCES))("%s uses no reserved word and no double underscore", (_n, src) => {
    const code = src.replace(/\/\*[\s\S]*?\*\/|\/\/.*$/gm, "");
    const ids = new Set(code.match(/\b[A-Za-z_]\w*\b/g));
    expect(RESERVED.filter((w) => ids.has(w))).toEqual([]);
    expect([...ids].filter((i) => i.includes("__"))).toEqual([]);
    expect(code.startsWith("#version 300 es")).toBe(true);
  });
  it("declares no unused plain texture", () => {
    expect(FIELD_FRAG).not.toContain("uPlain");
  });
});
