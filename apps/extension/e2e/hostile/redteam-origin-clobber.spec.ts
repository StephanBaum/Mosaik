/**
 * RT-EXT red team: a page that replaces `window.origin`, `top`, `parent`, `frameElement` and the
 * document's visibility in its own world. The content script reads them in the isolated world, so
 * on evil.test the fill is matched against evil.test (no match, no credential) and on bank.test the
 * replacements do not stop the fill.
 */
import { BANK, EVIL, bankItem, expect, expectFilled, expectNothingFilled, open, test } from "../fixtures.ts";

const PAGE = "/redteam-origin-clobber.html";

test("a page claiming the bank's origin through replaced globals gets no bank credential", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${EVIL}${PAGE}`);
  // The replacements are real in the page's world.
  expect(await page.evaluate(() => (window as unknown as { __clobbered: unknown }).__clobbered)).toEqual({
    origin: BANK,
    top: true,
    parent: true,
  });
  const tabId = await fake.tabIdOf(EVIL);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "noMatch" });
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1" })).toEqual({ ok: false, error: "originMismatch" });
  // Every request named the page's real origin, never the claimed one.
  const log = await fake.log();
  expect(log.filter((entry) => entry.origin !== null).map((entry) => entry.origin)).toEqual(
    log.filter((entry) => entry.origin !== null).map(() => EVIL),
  );
  expect(log.filter((entry) => entry.op === "fill")).toHaveLength(1);
  await expectNothingFilled(page);
});

test("the same replacements on the bank's own page do not stop the fill", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}${PAGE}`);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true, summary: { username: 1, password: 1 } });
  await expectFilled(page, BANK);
});
