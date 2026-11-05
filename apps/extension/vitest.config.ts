import { defineProject } from "vitest/config";

export default defineProject({
  test: {
    name: "extension",
    environment: "jsdom",
    include: ["src/**/*.test.ts", "scripts/**/*.test.ts"],
  },
});
