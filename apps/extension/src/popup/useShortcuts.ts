/**
 * Keys in the logins view: Enter fills, Ctrl+C copies the user name, Ctrl+Shift+C the password,
 * Ctrl+Alt+C the code (Command on a Mac), for the row that has focus, else the first match.
 * Text the user selected, or a text box, keeps its own copy and Enter. Esc is handled by the
 * menus and dialogs themselves (the top layer closes first).
 */
import { useEffect, useLayoutEffect, useRef } from "react";
import { useAnnounce } from "./components/Announcer.tsx";
import { useI18n } from "./i18n.tsx";
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
function copyFieldFor(event: KeyboardEvent, target: Element | null): ShortcutField | null {
  const field = fieldOf(event);
  const typing = (window.getSelection()?.toString() ?? "") !== "" || (target?.closest(OWN_COPY) ?? null) !== null;
  return typing ? null : field;
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
  const announce = useAnnounce();
  const { errorText } = useI18n();
  // The listener is added once and reads the latest props through a ref, committed together with
  // the render: re-adding it after every render left a window in which a key pressed right after
  // the logins appeared still saw the previous, empty list and did nothing.
  const latest = useRef({ items, fill, copy, enabled, announce, errorText });
  useLayoutEffect(() => {
    latest.current = { items, fill, copy, enabled, announce, errorText };
  });
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const { items: current, fill: doFill, copy: doCopy, enabled: on, announce: say, errorText: text } = latest.current;
      if (!on) {
        return;
      }
      const target = event.target instanceof Element ? event.target : null;
      const item = itemAt(target, current);
      if (item === undefined || event.defaultPrevented) {
        return;
      }
      if (isFillKey(event, target)) {
        if (item.fillable) {
          event.preventDefault();
          doFill(item);
        }
        return;
      }
      const field = copyFieldFor(event, target);
      if (field === "totp" && !item.hasOtp) {
        event.preventDefault();
        say(text("noOtp"));
      } else if (field !== null) {
        event.preventDefault();
        void doCopy({ kind: "item", vaultId: item.vaultId, itemId: item.itemId, field });
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);
}
