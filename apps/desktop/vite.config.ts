/**
 * Vite config for the desktop webview (ADR-0019).
 *
 * Modes: the default modes build the real app, whose IPC goes to Tauri; `--mode mock`
 * builds or serves it in a plain browser with the in-memory mock IPC
 * (src/ipc/mock.ts). Non-mock builds replace the mock with a stub, so no mock code or
 * sample data ships in the app.
 */
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import react from "@vitejs/plugin-react";
// By path, not by package name: Vite's config bundler leaves packages external, and
// Node cannot import the workspace package's TypeScript source.
import { cssVariables } from "../../packages/design-tokens/src/css.ts";
import { defineConfig, type Plugin } from "vite";

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

/** Keeps the mock IPC and its sample data out of every non-mock build. */
function excludeMockIpc(mode: string): Plugin {
  return {
    name: "mosaic-exclude-mock-ipc",
    apply: "build",
    enforce: "pre",
    load(id) {
      if (mode !== "mock" && /[\\/]src[\\/]ipc[\\/]mock\.ts$/.test(id)) {
        return [
          'export function createMockIpc() { throw new Error("The mock IPC is not part of this build."); }',
          "export function mockOptionsFromUrl() { return {}; }",
        ].join("\n");
      }
      return undefined;
    },
  };
}

/**
 * Ships the SIL OFL 1.1 texts with the bundled fonts (OFL condition 2) as
 * `licenses/<font>-OFL.txt` in the build output.
 */
function fontLicenses(): Plugin {
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

interface TauriConfig {
  app: { security: { csp: string } };
}

/** The production CSP from tauri.conf.json, for the mock preview server (Playwright). */
export function productionCsp(): string {
  const config = JSON.parse(
    readFileSync(new URL("./src-tauri/tauri.conf.json", import.meta.url), "utf8"),
  ) as TauriConfig;
  return config.app.security.csp;
}

export default defineConfig(({ mode }) => ({
  plugins: [react(), tokensStylesheet(), excludeMockIpc(mode), fontLicenses()],
  // Tauri expects a fixed port and shows its own errors.
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: "localhost" },
  preview: {
    port: 1421,
    strictPort: true,
    host: "localhost",
    headers: { "Content-Security-Policy": productionCsp() },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022",
    // Never inline assets as data: URIs; the CSP allows 'self' only.
    assetsInlineLimit: 0,
    sourcemap: false,
  },
}));
