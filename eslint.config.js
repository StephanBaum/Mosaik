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
  { files: ["**/*.js"], ...tseslint.configs.disableTypeChecked },
);
