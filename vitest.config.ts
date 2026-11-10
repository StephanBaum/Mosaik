import { configDefaults, defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Agent worktrees live under .claude/ and hold full copies of the repo.
    exclude: [...configDefaults.exclude, ".claude/**"],
    projects: [
      {
        test: {
          name: "packages",
          include: ["packages/*/src/**/*.test.ts"],
        },
      },
      {
        test: {
          // The security-key page the sync server serves (crates/server/assets/webauthn).
          name: "server-page",
          environment: "jsdom",
          include: ["crates/server/assets/webauthn/*.test.ts"],
        },
      },
      "apps/desktop/vitest.config.ts",
      "apps/extension/vitest.config.ts",
    ],
  },
});
