import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const stylesDir = dirname(fileURLToPath(import.meta.url));

describe("stylesheets", () => {
  it.each(readdirSync(stylesDir).filter((f) => f.endsWith(".css")))("%s has no colour literals", (file) => {
    const css = readFileSync(join(stylesDir, file), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    // rgb(var(--tint-rgb) / …) stays allowed.
    expect(css).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(\s*\d/i);
  });
});
