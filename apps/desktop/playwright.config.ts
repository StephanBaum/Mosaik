/**
 * End-to-end tests against a mock-mode production build in Chromium, served with the
 * app's production CSP (vite.config.ts `preview.headers`), so a CSP violation shows up
 * here before it shows up in Tauri.
 */
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  forbidOnly: Boolean(process.env["CI"]),
  retries: process.env["CI"] ? 1 : 0,
  reporter: process.env["CI"] ? "list" : "line",
  use: {
    baseURL: "http://localhost:1421",
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "pnpm build:mock && pnpm preview --mode mock",
    url: "http://localhost:1421",
    reuseExistingServer: !process.env["CI"],
    timeout: 120_000,
  },
});
