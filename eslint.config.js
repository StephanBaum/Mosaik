// @ts-check
import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

export default tseslint.config(
  // docs/design holds static design references (mockups, the holo prototype), not app code.
  {
    ignores: [
      "**/dist/**",
      "**/node_modules/**",
      "**/coverage/**",
      "target/**",
      "docs/design/**",
      // Agent worktrees are full repo copies; flat config does not read .gitignore.
      ".claude/**",
      // Tauri build output and Playwright artifacts.
      "apps/desktop/src-tauri/target/**",
      "apps/desktop/src-tauri/gen/**",
      "apps/desktop/test-results/**",
      "apps/desktop/playwright-report/**",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  {
    languageOptions: {
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
    rules: {
      "no-console": "error",
      "no-eval": "error",
      "no-implied-eval": "error",
      "no-new-func": "error",
    },
  },
  { files: ["apps/*/src/**/*.tsx", "apps/*/src/**/*.ts"], ...reactHooks.configs.flat.recommended },
  // Size and complexity limits for app and package code (tests and generated fixtures excluded),
  // so oversized or tangled code shows up while writing it, not in review.
  {
    files: ["apps/*/src/**/*.{ts,tsx}", "packages/*/src/**/*.ts"],
    ignores: ["**/*.test.{ts,tsx}", "**/e2e/**", "**/contract.fixtures.ts"],
    rules: {
      complexity: ["error", 12],
      "max-depth": ["error", 4],
      "max-params": ["error", 4],
      "max-lines-per-function": ["error", { max: 80, skipBlankLines: true, skipComments: true }],
      "max-lines": ["error", { max: 400, skipBlankLines: true, skipComments: true }],
    },
  },
  // TEMPORARY (M3): code that predates the limits. The M3 UI track replaces the screen and
  // mock files; the M3 integration removes this block and fixes whatever still fails.
  {
    files: [
      "apps/desktop/src/screens/VaultScreen.tsx",
      "apps/desktop/src/screens/OnboardingScreen.tsx",
      "apps/desktop/src/ipc/mock/generator.ts",
      "apps/desktop/src/ipc/mock/data.ts",
      "packages/i18n/src/index.ts",
    ],
    rules: { complexity: "off", "max-params": "off", "max-lines-per-function": "off" },
  },
  { files: ["**/*.js"], ...tseslint.configs.disableTypeChecked },
);
