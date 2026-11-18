/**
 * End-to-end tests of the built website: shaders, pages, fallbacks, transitions and runtime
 * budgets. They run against `vite preview` of `dist/`, so `pnpm --filter @mosaic/web build` comes
 * first. SwiftShader gives CI a software WebGL2; the fixtures lift the "major performance caveat"
 * refusal where a test needs the field to run.
 */
import { defineConfig } from "@playwright/test";

export const PORT = 5181;

export default defineConfig({
  testDir: "./e2e",
  workers: 1,
  timeout: 90_000,
  expect: { timeout: 10_000 },
  forbidOnly: Boolean(process.env["CI"]),
  // One retry on CI: the runtime budgets are medians of three cold loads on a shared runner.
  retries: process.env["CI"] ? 1 : 0,
  reporter: process.env["CI"] ? "list" : "line",
  webServer: {
    command: `pnpm exec vite preview --port ${String(PORT)} --strictPort`,
    url: `http://localhost:${String(PORT)}/`,
    reuseExistingServer: false,
    timeout: 60_000,
  },
  use: { baseURL: `http://localhost:${String(PORT)}` },
  projects: [
    {
      name: "chromium",
      use: {
        browserName: "chromium",
        launchOptions: { args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] },
      },
    },
  ],
});
