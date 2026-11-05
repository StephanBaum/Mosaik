import { useEffect, useRef } from "react";

/**
 * When the editor closes its form is gone and focus would fall to the page body. It goes back to
 * the open item's row, or to "New item" when no item is open (a cancelled new item).
 */
export function useEditorFocusReturn(open: boolean): void {
  const wasOpen = useRef(open);
  useEffect(() => {
    if (wasOpen.current && !open) {
      const target =
        document.querySelector<HTMLElement>('.item-row[aria-current="true"]') ??
        document.querySelector<HTMLElement>("[data-new-item]");
      target?.focus();
    }
    wasOpen.current = open;
  }, [open]);
}
