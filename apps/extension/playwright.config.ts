/**
 * End-to-end tests of the extension in Chromium with the `e2e` build (`pnpm build:e2e`): the
 * hostile-page suite in e2e/hostile. The browser, the hostile-site server and the fake desktop
 * are started by the fixtures in e2e/fixtures.ts; nothing is started here.
 */
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  workers: 1,
  timeout: 60_000,
  expect: { timeout: 10_000 },
  forbidOnly: Boolean(process.env["CI"]),
  retries: process.env["CI"] ? 1 : 0,
  reporter: process.env["CI"] ? "list" : "line",
  projects: [{ name: "chromium" }],
});
