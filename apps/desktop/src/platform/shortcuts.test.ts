import { formatMessage, type Locale, type MessageKey } from "@mosaic/i18n";
import { describe, expect, it } from "vitest";
import { ariaKeyshortcuts, format, GLOBAL_PALETTE, matches, SHORTCUTS } from "./shortcuts.ts";

const tr = (locale: Locale) => (key: MessageKey) => formatMessage(locale, key);

function press(init: KeyboardEventInit): KeyboardEvent {
  return new KeyboardEvent("keydown", init);
}

describe("matches", () => {
  it("uses Ctrl on Windows and Linux, ⌘ on macOS", () => {
    expect(matches(press({ key: "n", ctrlKey: true }), SHORTCUTS.newItem, "windows")).toBe(true);
    expect(matches(press({ key: "n", ctrlKey: true }), SHORTCUTS.newItem, "linux")).toBe(true);
    expect(matches(press({ key: "n", metaKey: true }), SHORTCUTS.newItem, "windows")).toBe(false);
    expect(matches(press({ key: "n", metaKey: true }), SHORTCUTS.newItem, "mac")).toBe(true);
    expect(matches(press({ key: "n", ctrlKey: true }), SHORTCUTS.newItem, "mac")).toBe(false);
  });

  it("is case-insensitive for key shortcuts and exact about Shift and Alt", () => {
    expect(matches(press({ key: "C", ctrlKey: true, shiftKey: true }), SHORTCUTS.copyPassword, "windows")).toBe(true);
    expect(matches(press({ key: "c", ctrlKey: true }), SHORTCUTS.copyPassword, "windows")).toBe(false);
    expect(matches(press({ key: "c", ctrlKey: true, shiftKey: true }), SHORTCUTS.copyUsername, "windows")).toBe(false);
    expect(matches(press({ key: "n", ctrlKey: true, altKey: true }), SHORTCUTS.newItem, "windows")).toBe(false);
  });

  it("compares event.code for Alt combinations, since ⌥ changes event.key on macOS", () => {
    const optionL = press({ key: "¬", code: "KeyL", metaKey: true, altKey: true });
    expect(matches(optionL, SHORTCUTS.lock, "mac")).toBe(true);
    expect(matches(press({ key: "l", code: "KeyL", ctrlKey: true, altKey: true }), SHORTCUTS.lock, "windows")).toBe(true);
    expect(matches(press({ key: "l", code: "KeyL", ctrlKey: true }), SHORTCUTS.lock, "windows")).toBe(false);
  });
});

describe("format", () => {
  it("uses symbols on macOS", () => {
    const t = tr("en");
    expect(format(SHORTCUTS.newItem, "mac", t)).toBe("⌘N");
    expect(format(SHORTCUTS.lock, "mac", t)).toBe("⌥⌘L");
    expect(format(SHORTCUTS.copyPassword, "mac", t)).toBe("⇧⌘C");
    expect(format(GLOBAL_PALETTE.mac, "mac", t)).toBe("⌥⌘Space");
  });

  it("uses key names elsewhere, localized", () => {
    expect(format(SHORTCUTS.newItem, "windows", tr("en"))).toBe("Ctrl+N");
    expect(format(SHORTCUTS.newItem, "windows", tr("de"))).toBe("Strg+N");
    expect(format(SHORTCUTS.lock, "linux", tr("en"))).toBe("Ctrl+Alt+L");
    expect(format(SHORTCUTS.settings, "windows", tr("en"))).toBe("Ctrl+,");
    expect(format(GLOBAL_PALETTE.windows, "windows", tr("en"))).toBe("Ctrl+Shift+Space");
    expect(format(GLOBAL_PALETTE.linux, "linux", tr("de"))).toBe("Strg+Umschalt+Leertaste");
  });
});

describe("ariaKeyshortcuts", () => {
  it("names the modifiers the way ARIA does", () => {
    expect(ariaKeyshortcuts(SHORTCUTS.newItem, "windows")).toBe("Control+N");
    expect(ariaKeyshortcuts(SHORTCUTS.lock, "mac")).toBe("Meta+Alt+L");
    expect(ariaKeyshortcuts(SHORTCUTS.copyPassword, "windows")).toBe("Control+Shift+C");
  });
});
