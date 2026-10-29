/**
 * In-app keyboard shortcuts (UX_GUIDELINES §4): one table for matching, display and
 * `aria-keyshortcuts`. `mod` is ⌘ on macOS and Ctrl elsewhere.
 */
import { useEffect } from "react";
import type { MessageKey } from "@mosaic/i18n";
import type { Platform } from "./platform.ts";

export interface Shortcut {
  /** Compared with `event.key` (lower case). */
  key?: string;
  /** Compared with `event.code`; used with Alt, because ⌥ changes `event.key` on macOS. */
  code?: string;
  mod: true;
  shift?: boolean;
  alt?: boolean;
}

export const SHORTCUTS = {
  palette: { key: "k", mod: true },
  newItem: { key: "n", mod: true },
  settings: { key: ",", mod: true },
  lock: { code: "KeyL", mod: true, alt: true },
  copyUsername: { key: "c", mod: true },
  copyPassword: { key: "c", mod: true, shift: true },
  copyTotp: { code: "KeyC", mod: true, alt: true },
} as const satisfies Record<string, Shortcut>;

export type ShortcutName = keyof typeof SHORTCUTS;

/**
 * The system-wide quick search shortcut (registered by Rust, `shortcut.rs`; it cannot
 * be remapped yet, M3-R10). It differs by platform, so it is a table of its own.
 */
export const GLOBAL_PALETTE = {
  mac: { code: "Space", mod: true, alt: true },
  windows: { code: "Space", mod: true, shift: true },
  linux: { code: "Space", mod: true, shift: true },
} as const satisfies Record<Platform, Shortcut>;

type Translate = (key: MessageKey) => string;

/** Whether `event` is `shortcut` on `platform` (no other modifier may be held). */
export function matches(event: KeyboardEvent, shortcut: Shortcut, platform: Platform): boolean {
  const mac = platform === "mac";
  const mod = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
  if (!mod || event.shiftKey !== (shortcut.shift === true) || event.altKey !== (shortcut.alt === true)) {
    return false;
  }
  if (shortcut.code !== undefined) {
    return event.code === shortcut.code;
  }
  return event.key.toLowerCase() === shortcut.key;
}

function keyLabel(shortcut: Shortcut, t: Translate): string {
  if (shortcut.code === "Space") {
    return t("keys.space");
  }
  if (shortcut.code?.startsWith("Key") === true) {
    return shortcut.code.slice(3);
  }
  return (shortcut.key ?? "").toUpperCase();
}

/** The shortcut as shown in the UI: ⌥⌘N on macOS, "Ctrl+Alt+N" or "Strg+Alt+N" elsewhere. */
export function format(shortcut: Shortcut, platform: Platform, t: Translate): string {
  const key = keyLabel(shortcut, t);
  if (platform === "mac") {
    return `${shortcut.alt === true ? "⌥" : ""}${shortcut.shift === true ? "⇧" : ""}⌘${key}`;
  }
  return [
    t("keys.ctrl"),
    ...(shortcut.shift === true ? [t("keys.shift")] : []),
    ...(shortcut.alt === true ? [t("keys.alt")] : []),
    key,
  ].join("+");
}

/** The value of `aria-keyshortcuts` (W3C modifier names; Meta on macOS, Control elsewhere). */
export function ariaKeyshortcuts(shortcut: Shortcut, platform: Platform): string {
  const key = shortcut.code === "Space" ? "Space" : keyLabel(shortcut, (k) => k);
  return [
    platform === "mac" ? "Meta" : "Control",
    ...(shortcut.shift === true ? ["Shift"] : []),
    ...(shortcut.alt === true ? ["Alt"] : []),
    key,
  ].join("+");
}

/** Runs `handlers[name]` when its shortcut is pressed (default prevented). */
export function useAppShortcuts(
  platform: Platform,
  handlers: Partial<Record<ShortcutName, () => void>>,
): void {
  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      for (const name of Object.keys(handlers) as ShortcutName[]) {
        const handler = handlers[name];
        if (handler !== undefined && matches(event, SHORTCUTS[name], platform)) {
          event.preventDefault();
          handler();
          return;
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [handlers, platform]);
}
