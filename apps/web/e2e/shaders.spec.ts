/** The four shader programs compile and link on a real WebGL2 driver (SwiftShader in CI) with empty info logs. */
import { expect, test } from "@playwright/test";
import { FRAG, VERT } from "../src/fx/shader.ts";
import { FIELD_FRAG, FIELD_VERT } from "../src/field/shaders.ts";

const PROGRAMS = {
  field: { vert: FIELD_VERT, frag: FIELD_FRAG },
  holo: { vert: VERT, frag: FRAG },
};

for (const [name, src] of Object.entries(PROGRAMS)) {
  test(`${name} shader compiles and links with empty logs`, async ({ page }) => {
    await page.goto("/");
    const r = await page.evaluate(({ vert, frag }) => {
      const gl = document.createElement("canvas").getContext("webgl2");
      if (gl === null) {
        return { webgl2: false } as const;
      }
      const sh = (type: number, text: string) => {
        const s = gl.createShader(type);
        if (s === null) {
          throw new Error("createShader");
        }
        gl.shaderSource(s, text);
        gl.compileShader(s);
        return s;
      };
      const vs = sh(gl.VERTEX_SHADER, vert);
      const fs = sh(gl.FRAGMENT_SHADER, frag);
      const p = gl.createProgram();
      gl.attachShader(p, vs);
      gl.attachShader(p, fs);
      gl.linkProgram(p);
      return {
        webgl2: true,
        vsOk: gl.getShaderParameter(vs, gl.COMPILE_STATUS) as boolean,
        fsOk: gl.getShaderParameter(fs, gl.COMPILE_STATUS) as boolean,
        linked: gl.getProgramParameter(p, gl.LINK_STATUS) as boolean,
        logs: [gl.getShaderInfoLog(vs), gl.getShaderInfoLog(fs), gl.getProgramInfoLog(p)],
      } as const;
    }, src);
    // WebGL2 must exist: a missing context is a failure, never a skip.
    expect(r.webgl2, "WebGL2 (SwiftShader) is available").toBe(true);
    if (r.webgl2) {
      expect(r.vsOk).toBe(true);
      expect(r.fsOk).toBe(true);
      expect(r.linked).toBe(true);
      expect(r.logs.map((l) => (l ?? "").trim())).toEqual(["", "", ""]);
    }
  });
}
