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
      "apps/desktop/vitest.config.ts",
    ],
  },
});
