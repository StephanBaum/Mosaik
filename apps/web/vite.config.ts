/**
 * Vite config for the landing page (design concept §5). React renders the pages at build time
 * only (scripts/prerender.ts); the browser gets HTML, one stylesheet and vanilla-TS islands.
 * The dev server answers on 5180; `?tune=1` there mounts the mosaic tuning panel, which is not
 * part of production builds.
 */
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { defineConfig, type Plugin } from "vite";
// By path, not by package name: Vite's config bundler leaves packages external, and Node
// cannot import the workspace packages' TypeScript source.
import { cssVariables } from "../../packages/design-tokens/src/css.ts";
import { web } from "../../packages/i18n/src/catalog/en/web.ts";
import { prerender } from "./scripts/prerender.ts";
import { headersFile, SECURITY_HEADERS } from "./src/pages/security.ts";

const TOKENS_ID = "virtual:mosaic-tokens.css";
const RESOLVED_TOKENS_ID = `\0${TOKENS_ID}`;

/** Serves the design tokens as a static stylesheet (no inline styles under the CSP). */
function tokensStylesheet(): Plugin {
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

/** Ships the SIL OFL 1.1 texts with the self-hosted fonts (OFL condition 2). */
function fontLicenses(): Plugin {
  const require = createRequire(import.meta.url);
  const fonts = [
    ["@fontsource-variable/dm-sans", "DM-Sans"],
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

export default defineConfig({
  appType: "mpa",
  plugins: [
    tokensStylesheet(),
    fontLicenses(),
    prerender({ ssrPlugins: [tokensStylesheet()], domain: web["web.site.domain"], headers: headersFile(), tokensCss: cssVariables }),
  ],
  server: { port: 5180, strictPort: true, host: "localhost" },
  preview: { port: 5181, strictPort: true, host: "localhost", headers: { ...SECURITY_HEADERS } },
  build: {
    target: "es2022",
    manifest: true,
    // Never inline assets as data: URIs; the CSP allows 'self' only.
    assetsInlineLimit: 0,
    modulePreload: { polyfill: false },
    sourcemap: false,
    rolldownOptions: { input: { main: "src/main.ts" } },
  },
});
