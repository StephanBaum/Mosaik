import { BANK, bankItem, expect, expectNothingFilled, open, test } from "../fixtures.ts";

test("fields under a transparent full-page overlay are not filled", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/overlay-occlusion.html`);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "noFields" });
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1" })).toEqual({ ok: false, error: "noFields" });
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);

  // Control: without the overlay the same fields are filled.
  await page.evaluate(() => {
    document.getElementById("overlay")?.remove();
  });
  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true });
});
