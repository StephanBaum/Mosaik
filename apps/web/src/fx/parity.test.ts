/**
 * Shader parity (design concept §3 M1, §5 shared code): the copies in src/fx are the desktop
 * holo sources byte for byte below their provenance header, and the mosaic shader carries the
 * desktop `foil()` and `wavelet()` unchanged.
 */
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { foilLibrary, MOSAIC_FRAG } from "../hero/mosaic-shader.ts";

const desktop = (name: string): string =>
  readFileSync(new URL(`../../../desktop/src/holo/${name}`, import.meta.url), "utf8");
const copy = (name: string): string => readFileSync(new URL(`./${name}`, import.meta.url), "utf8");

/** Drops the leading `// Provenance` comment block and the blank line after it. */
function withoutProvenance(source: string): string {
  const lines = source.split(/\r?\n/);
  expect(lines[0]).toMatch(/^\/\/ Provenance: verbatim copy of apps\/desktop\/src\/holo\//);
  const body = lines.findIndex((line) => line.trim() === "");
  return lines.slice(body + 1).join("\n");
}

const normalise = (source: string): string => source.replace(/\r\n/g, "\n");

/** The text of the GLSL function that starts with `signature`, up to its closing brace. */
function glslFunction(source: string, signature: string): string {
  const start = source.indexOf(signature);
  expect(start, signature).toBeGreaterThanOrEqual(0);
  let depth = 0;
  for (let i = source.indexOf("{", start); i < source.length; i++) {
    depth += source[i] === "{" ? 1 : source[i] === "}" ? -1 : 0;
    if (depth === 0) {
      return source.slice(start, i + 1);
    }
  }
  throw new Error(`unterminated ${signature}`);
}

describe("fx copies", () => {
  it.each(["shader.ts", "effects.ts", "motion.ts", "gl.ts", "draw.ts"])("%s matches the desktop source", (name) => {
    expect(normalise(withoutProvenance(copy(name)))).toBe(normalise(desktop(name)));
  });
});

describe("mosaic shader", () => {
  const desktopFrag = normalise(desktop("shader.ts"));

  it.each(["vec3 foil(", "vec3 wavelet(", "void stripe(", "vec3 thinFilm(", "float fbm3(", "float hash12("])(
    "carries the desktop %s verbatim",
    (signature) => {
      expect(MOSAIC_FRAG).toContain(glslFunction(desktopFrag, signature));
    },
  );

  it("embeds the whole foil library of the desktop shader", () => {
    const library = foilLibrary(desktopFrag.slice(desktopFrag.indexOf("export const FRAG")));
    expect(library).toContain("vec3 foil(");
    expect(library).toContain("vec3 wavelet(");
    expect(MOSAIC_FRAG).toContain(library);
  });

  it("fails loudly when the library markers move", () => {
    expect(() => foilLibrary("void main(){}")).toThrow(/markers/);
  });
});
