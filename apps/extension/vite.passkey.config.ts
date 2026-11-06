/**
 * The two passkey scripts, each as one IIFE (registered at run time, never in the manifest),
 * into the same `dist/<mode>`: `passkey-main.js` (the shim, main world) and `passkey-bridge.js`
 * (isolated world). A library build has one entry per IIFE, so the main-world script is built
 * here and the bridge by a nested build once it is written.
 */
import { resolve } from "node:path";
import { build, defineConfig, type Plugin, type UserConfig } from "vite";

const entry = (name: "main" | "bridge", mode: string): UserConfig => ({
  clearScreen: false,
  logLevel: "warn",
  build: {
    outDir: `dist/${mode}`,
    emptyOutDir: false,
    target: "es2022",
    minify: false,
    sourcemap: false,
    lib: {
      entry: resolve(import.meta.dirname, `src/passkey/${name}-entry.ts`),
      formats: ["iife"],
      name: name === "main" ? "mosaicPasskeyMain" : "mosaicPasskeyBridge",
      fileName: () => `passkey-${name}.js`,
    },
  },
});

function bridgeAfterMain(mode: string): Plugin {
  return {
    name: "mosaic-passkey-bridge",
    apply: "build",
    async closeBundle() {
      await build({ configFile: false, ...entry("bridge", mode) });
    },
  };
}

export default defineConfig(({ mode }) => ({ ...entry("main", mode), plugins: [bridgeAfterMain(mode)] }));
