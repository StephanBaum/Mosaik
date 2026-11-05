import type { FillOutcome } from "../../src/background/fill.ts";
import { BANK, bankItem, expect, expectNothingFilled, open, test } from "../fixtures.ts";

const errorOf = (outcome: FillOutcome | null): string => (outcome !== null && !outcome.ok ? outcome.error : "none");

test("a login page served with a CSP sandbox (opaque origin, site URL) is not filled", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/csp-sandbox-login.html`);
  // The page's scripts really run in an opaque origin, although the tab URL names the site.
  expect(await page.evaluate(() => (window as unknown as { __realOrigin: string }).__realOrigin)).toBe("null");
  const tabId = await fake.tabIdOf(BANK);

  for (const gesture of [
    { kind: "shortcut", tabId },
    { kind: "popupFill", tabId, itemId: "i1", confirmFrame: true },
  ] as const) {
    // The content script reports the document's own origin ("null"), not the URL's.
    expect(errorOf(await fake.gesture(gesture))).toBe("frameOpaque");
  }
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
});

test("the site's own page in a sandboxed frame (opaque origin, site URL) is not filled", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/sandboxed-src-frame.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  for (const gesture of [
    { kind: "shortcut", tabId },
    { kind: "popupFill", tabId, itemId: "i1", confirmFrame: true },
  ] as const) {
    expect(errorOf(await fake.gesture(gesture))).toBe("frameOpaque");
  }
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
});
