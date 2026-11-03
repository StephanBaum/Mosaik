import { useMediaQuery } from "./useMediaQuery.ts";

/** Whether the sidebar is the icon rail (the 900 px breakpoint of `styles/shell.css`). */
export function useRail(): boolean {
  return useMediaQuery("(max-width: 900px)");
}
