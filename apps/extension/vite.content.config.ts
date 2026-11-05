/** The content script as one IIFE (content scripts are classic scripts), into the same `dist/<mode>`. */
import { resolve } from "node:path";
import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
  clearScreen: false,
  build: {
    outDir: `dist/${mode}`,
    emptyOutDir: false,
    target: "es2022",
    minify: false,
    sourcemap: false,
    lib: {
      entry: resolve(import.meta.dirname, "src/content/index.ts"),
      formats: ["iife"],
      name: "mosaicContentBundle",
      fileName: () => "content.js",
    },
  },
}));
