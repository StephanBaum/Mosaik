/**
 * Build-time subset of DM Sans to the characters the site prints (ADR in Task 18, decision 4).
 * The variable font keeps its `opsz` axis and a clamped `wght` axis (400-700, every weight the stylesheets use); the glyph set shrinks. IBM Plex Mono is
 * never subset: its OFL carries the Reserved Font Name "Plex", so a modified file could not keep
 * the name. Runs in vite.config.ts, so the catalogs are imported by path (the config bundler
 * leaves workspace packages external and Node cannot load their TypeScript).
 */
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import subsetFont from "subset-font";
import type { Plugin } from "vite";
import { de } from "../../../packages/i18n/src/de.ts";
import { en } from "../../../packages/i18n/src/en.ts";

/** Typographic characters the site uses that no catalog string has to contain. */
export const EXTRA_CHARS = "‘’‚“”„–—…·→←✓✕●›‹×€°§0123456789abcdef";

const here = dirname(fileURLToPath(import.meta.url));
export const CACHE_DIR = join(here, "..", "node_modules", ".cache", "mosaic-fonts");
export const SUBSET_FILE = join(CACHE_DIR, "dm-sans-sub.woff2");
const HASH_FILE = join(CACHE_DIR, "charset.sha256");

/** Sorted unique characters: Basic Latin, `EXTRA_CHARS` and every `web.*` message of both languages. */
export function webCharset(): string {
  const chars = new Set<string>();
  for (let code = 0x20; code <= 0x7e; code++) {
    chars.add(String.fromCharCode(code));
  }
  for (const c of EXTRA_CHARS) {
    chars.add(c);
  }
  for (const catalog of [en, de] as readonly Readonly<Record<string, string>>[]) {
    for (const [key, value] of Object.entries(catalog)) {
      if (key.startsWith("web.")) {
        // Argument names ({name}, {domain}, …) are filled at build time; their literal text stays.
        for (const c of value.replace(/\{[a-zA-Z]+\}/g, "")) {
          chars.add(c);
        }
      }
    }
  }
  return [...chars].sort().join("");
}

function sourceFont(): Buffer {
  const require = createRequire(import.meta.url);
  const pkgDir = dirname(require.resolve("@fontsource-variable/dm-sans/package.json"));
  return readFileSync(join(pkgDir, "files", "dm-sans-latin-opsz-normal.woff2"));
}

/** The weights the site uses lie in 400-700; clamping the `wght` axis to that range halves the file. */
export const WGHT_RANGE = { min: 400, max: 700 } as const;

/** DM Sans (latin, opsz) reduced to `text`, as woff2; `opsz` kept, `wght` clamped to `WGHT_RANGE`. */
export function subsetDmSans(text: string): Promise<Buffer> {
  return subsetFont(sourceFont(), text, { targetFormat: "woff2", variationAxes: { wght: WGHT_RANGE } });
}

/** The hash stored with the cached subset, or none (read once, no check-then-read race). */
function cachedHash(): string | null {
  try {
    return readFileSync(HASH_FILE, "utf8");
  } catch {
    return null;
  }
}

/** Writes the subset to `node_modules/.cache/mosaic-fonts` when the charset changed. */
export function fontSubset(): Plugin {
  return {
    name: "mosaic-font-subset",
    async buildStart() {
      const text = webCharset();
      const hash = createHash("sha256").update(text).update(JSON.stringify(WGHT_RANGE)).digest("hex");
      if (cachedHash() === hash && existsSync(SUBSET_FILE)) {
        return;
      }
      mkdirSync(CACHE_DIR, { recursive: true });
      writeFileSync(SUBSET_FILE, await subsetDmSans(text));
      writeFileSync(HASH_FILE, hash);
    },
  };
}
