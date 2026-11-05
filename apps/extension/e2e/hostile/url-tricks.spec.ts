import { BANK, bankItem, expect, expectFilled, inspect, open, test } from "../fixtures.ts";

test("pushState to another path keeps working; a tab replaced by a blob: document is unsupported", async ({
  page,
  fake,
}) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/url-tricks.html`);
  const tabId = await fake.tabIdOf(BANK);

  // Same origin, other path: the tab URL changes, the origin does not.
  await page.evaluate(() => {
    history.pushState({}, "", "/elsewhere.html");
  });
  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true });
  await expectFilled(page, `${BANK}/elsewhere.html`);

  // The page swaps itself for a blob: document with a login form.
  await page.evaluate(() => {
    (window as unknown as { toBlob: () => void }).toBlob();
  });
  await expect.poll(() => page.url().startsWith("blob:")).toBe(true);
  await page.waitForLoadState("load");
  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "unsupportedPage" });
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1" })).toEqual({ ok: false, error: "unsupportedPage" });
  const [frame] = await inspect(page);
  expect(frame?.values).toEqual({ user: "", pass: "" });
  expect(await fake.fills()).toHaveLength(1);
});
