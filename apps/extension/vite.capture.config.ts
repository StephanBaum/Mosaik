/** The opt-in save capture script as one IIFE (registered at run time, never in the manifest), into the same `dist/<mode>`. */
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
      entry: resolve(import.meta.dirname, "src/capture/index.ts"),
      formats: ["iife"],
      name: "mosaicCaptureBundle",
      fileName: () => "capture.js",
    },
  },
}));
