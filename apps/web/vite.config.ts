/**
 * Vite config for the website (design concept §5, website v2 §1.4). React renders the pages at build time
 * only (scripts/prerender.ts); the browser gets HTML, one stylesheet and vanilla-TS islands.
 * The dev server answers on 5180.
 */
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { defineConfig, type Plugin } from "vite";
// By path, not by package name: Vite's config bundler leaves packages external, and Node
// cannot import the workspace packages' TypeScript source.
import { cssVariables } from "../../packages/design-tokens/src/css.ts";
import { webCssVariables } from "../../packages/design-tokens/src/web.ts";
import { landing } from "../../packages/i18n/src/catalog/en/web/landing.ts";
import { fontSubset } from "./scripts/fonts.ts";
import { prerender } from "./scripts/prerender.ts";
import { SOURCE_URLS } from "./src/pages/compareData.ts";
import { headersFile, SECURITY_HEADERS, SPECULATION_PATH, speculationRules } from "./src/pages/security.ts";
import { LINKS, site as flags } from "./src/pages/site.ts";

/** The app tokens plus the site palette, as one stylesheet. */
const tokensCss = (): string => `${cssVariables()}

${webCssVariables()}`;

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
      return id === RESOLVED_TOKENS_ID ? tokensCss() : undefined;
    },
  };
}

/** `vite preview` has no `_headers`: serve the speculation rules with their media type. */
function previewSpeculation(): Plugin {
  return {
    name: "mosaic-preview-speculation",
    apply: "serve",
    configurePreviewServer(server) {
      server.middlewares.use(SPECULATION_PATH, (_req, res) => {
        res.setHeader("Content-Type", "application/speculationrules+json");
        res.setHeader("Cache-Control", "public, max-age=3600");
        res.end(speculationRules());
      });
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
    fontSubset(),
    tokensStylesheet(),
    fontLicenses(),
    previewSpeculation(),
    prerender({
      ssrPlugins: [tokensStylesheet()],
      domain: landing["web.site.domain"],
      headers: headersFile(),
      sources: [...SOURCE_URLS, ...Object.values(LINKS)],
      legalReady: flags.legalReady,
      tokensCss,
    }),
  ],
  server: { port: 5180, strictPort: true, host: "localhost" },
  // `vite preview` serves no `_headers`: the CSP set and the speculation rules header are sent from here.
  preview: { port: 5181, strictPort: true, host: "localhost", headers: { ...SECURITY_HEADERS, "Speculation-Rules": `"${SPECULATION_PATH}"` } },
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
