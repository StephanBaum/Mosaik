import react from "@vitejs/plugin-react";
import { defineProject } from "vitest/config";

export default defineProject({
  plugins: [react()],
  test: {
    name: "desktop",
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["./src/test/setup.ts"],
    // Most tests render the whole app in jsdom: with the files running in parallel on a loaded
    // machine a render plus typed input takes more than the default 5 s (the suite then failed
    // 29 tests that pass alone in about a second each).
    testTimeout: 20_000,
  },
});
