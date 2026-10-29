import { useMediaQuery } from "./useMediaQuery.ts";
import type { Theme } from "../ipc/contract/index.ts";

/** The theme in effect: the setting, or the OS preference for "system". */
export function useEffectiveTheme(theme: Theme): "light" | "dark" {
  const prefersDark = useMediaQuery("(prefers-color-scheme: dark)");
  if (theme === "system") {
    return prefersDark ? "dark" : "light";
  }
  return theme;
}
