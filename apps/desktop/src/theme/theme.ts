/**
 * Theme selection. The colors themselves come from the token stylesheet
 * (`@mosaic/design-tokens/css`): `data-theme` forces light or dark, no attribute follows
 * the OS (the default, "system").
 */
import type { Theme } from "../ipc/contract/index.ts";

export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  if (theme === "system") {
    delete root.dataset["theme"];
  } else {
    root.dataset["theme"] = theme;
  }
}
