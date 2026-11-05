/** Vite plugins shared by the extension builds. */
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import type { Plugin } from "vite";
import { cssVariables } from "../../../packages/design-tokens/src/css.ts";
import { ICON_SIZES, renderIcon } from "./icons.ts";
import { localeMessages, manifest, targetOf, type BuildMode } from "./manifest.ts";

const TOKENS_ID = "virtual:mosaic-tokens.css";
const RESOLVED_TOKENS_ID = `\0${TOKENS_ID}`;

/** Serves the design tokens as a static stylesheet (the extension-page CSP allows no inline styles). */
export function tokensStylesheet(): Plugin {
  return {
    name: "mosaic-tokens-stylesheet",
    resolveId(id) {
      return id === TOKENS_ID ? RESOLVED_TOKENS_ID : undefined;
    },
    load(id) {
      return id === RESOLVED_TOKENS_ID ? cssVariables() : undefined;
    },
  };
}

/** Emits `manifest.json`, `_locales/*` and the icons next to the bundles. */
export function extensionAssets(mode: BuildMode): Plugin {
  return {
    name: "mosaic-extension-assets",
    apply: "build",
    generateBundle() {
      const json = (value: unknown): string => `${JSON.stringify(value, null, 2)}\n`;
      this.emitFile({ type: "asset", fileName: "manifest.json", source: json(manifest(targetOf(mode), mode)) });
      for (const locale of ["en", "de"] as const) {
        this.emitFile({
          type: "asset",
          fileName: `_locales/${locale}/messages.json`,
          source: json(localeMessages(locale)),
        });
      }
      for (const size of ICON_SIZES) {
        this.emitFile({ type: "asset", fileName: `icons/icon-${String(size)}.png`, source: renderIcon(size) });
      }
    },
  };
}

/**
 * Replaces `src/background/e2e.ts` (the fake desktop and the test hooks) with its stub in
 * every build except `--mode e2e`, so none of it ships. `check-dist.mjs` double-checks.
 */
export function excludeE2e(mode: string): Plugin {
  const stub = readFileSync(join(import.meta.dirname, "../src/background/e2e.stub.ts"), "utf8");
  return {
    name: "mosaic-exclude-e2e",
    apply: "build",
    enforce: "pre",
    load(id) {
      return mode !== "e2e" && /[/\\]src[/\\]background[/\\]e2e\.ts$/.test(id) ? stub : undefined;
    },
  };
}

/** Ships the SIL OFL 1.1 texts with the bundled fonts (OFL condition 2) as `licenses/<font>-OFL.txt`. */
export function fontLicenses(): Plugin {
  const require = createRequire(import.meta.url);
  const fonts = [
    ["@fontsource/dm-sans", "DM-Sans"],
    ["@fontsource/ibm-plex-mono", "IBM-Plex-Mono"],
  ] as const;
  return {
    name: "mosaic-font-licenses",
    apply: "build",
    generateBundle() {
      for (const [pkg, name] of fonts) {
        this.emitFile({
          type: "asset",
          fileName: `licenses/${name}-OFL.txt`,
          source: readFileSync(require.resolve(`${pkg}/LICENSE`), "utf8"),
        });
      }
    },
  };
}
