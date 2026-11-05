/**
 * Keys in the logins view: Enter fills, Ctrl+C copies the user name, Ctrl+Shift+C the password,
 * Ctrl+Alt+C the code (Command on a Mac), for the row that has focus, else the first match.
 * Text the user selected, or a text box, keeps its own copy and Enter. Esc is handled by the
 * menus and dialogs themselves (the top layer closes first).
 */
import { useEffect } from "react";
import type { Actions, ItemKey } from "./useActions.ts";

export interface ShortcutItem extends ItemKey {
  hasOtp: boolean;
  fillable: boolean;
}

const OWN_ENTER = "button, a, input, textarea, select, summary, [role^='menuitem']";
const OWN_COPY = "input, textarea, select, [contenteditable='true']";

type ShortcutField = "username" | "password" | "totp";

function fieldOf(event: KeyboardEvent): ShortcutField | null {
  if (event.key.toLowerCase() !== "c" || !(event.ctrlKey || event.metaKey)) {
    return null;
  }
  if (event.altKey) {
    return "totp";
  }
  return event.shiftKey ? "password" : "username";
}

function itemAt(target: Element | null, items: readonly ShortcutItem[]): ShortcutItem | undefined {
  const row = target?.closest<HTMLElement>("[data-item-id]") ?? null;
  const found =
    row === null
      ? undefined
      : items.find((each) => each.vaultId === row.dataset["vaultId"] && each.itemId === row.dataset["itemId"]);
  return found ?? items[0];
}

/** Enter on the page or a row (not on a control that has its own Enter) fills. */
function isFillKey(event: KeyboardEvent, target: Element | null): boolean {
  return (
    event.key === "Enter" &&
    !event.shiftKey &&
    !event.ctrlKey &&
    !event.altKey &&
    (target === null || target.closest(OWN_ENTER) === null)
  );
}

/** The copy shortcut's field, unless the user is copying text or typing in a box. */
function copyFieldFor(event: KeyboardEvent, target: Element | null, item: ShortcutItem): ShortcutField | null {
  const field = fieldOf(event);
  const typing = (window.getSelection()?.toString() ?? "") !== "" || (target?.closest(OWN_COPY) ?? null) !== null;
  return field === null || typing || (field === "totp" && !item.hasOtp) ? null : field;
}

export function useShortcuts({
  items,
  fill,
  copy,
  enabled,
}: {
  items: readonly ShortcutItem[];
  fill: (item: ShortcutItem) => void;
  copy: Actions["copy"];
  enabled: boolean;
}): void {
  useEffect(() => {
    if (!enabled) {
      return undefined;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target instanceof Element ? event.target : null;
      const item = itemAt(target, items);
      if (item === undefined || event.defaultPrevented) {
        return;
      }
      if (isFillKey(event, target)) {
        if (item.fillable) {
          event.preventDefault();
          fill(item);
        }
        return;
      }
      const field = copyFieldFor(event, target, item);
      if (field !== null) {
        event.preventDefault();
        void copy({ kind: "item", vaultId: item.vaultId, itemId: item.itemId, field });
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [items, fill, copy, enabled]);
}
