/**
 * Popup (HTML) and background (ES module) of the extension. `--mode chromium | firefox` build
 * the two stores' packages into `dist/<mode>`; `--mode dev` is chromium with the dev key and
 * `--mode e2e` adds the test hooks (see scripts/plugins.ts). The content script is a separate
 * IIFE build (vite.content.config.ts) because content scripts cannot be ES modules.
 */
import { resolve } from "node:path";
import { defineConfig } from "vite";
import type { BuildMode } from "./scripts/manifest.ts";
import { excludeE2e, extensionAssets, tokensStylesheet } from "./scripts/plugins.ts";

export default defineConfig(({ mode }) => ({
  plugins: [tokensStylesheet(), excludeE2e(mode), extensionAssets(mode as BuildMode)],
  clearScreen: false,
  build: {
    outDir: `dist/${mode}`,
    emptyOutDir: true,
    target: "es2022",
    // Readable output: store reviewers read the package, and nothing is inlined or minified away.
    minify: false,
    sourcemap: false,
    modulePreload: false,
    assetsInlineLimit: 0,
    rolldownOptions: {
      input: {
        popup: resolve(import.meta.dirname, "popup.html"),
        background: resolve(import.meta.dirname, "src/background/index.ts"),
      },
      output: {
        entryFileNames: "[name].js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
      },
    },
  },
}));
