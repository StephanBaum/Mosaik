import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
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
