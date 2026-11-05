import { BANK, PASSWORD, bankItem, expect, expectFilled, expectNothingFilled, open, test } from "../fixtures.ts";

test("a page that waits for an autofill gets nothing on load, and the credential only after a gesture", async ({
  page,
  fake,
}) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/autofill-on-load.html`);
  await page.waitForTimeout(1500);

  // No gesture: no value in any field, nothing seen by the page's scripts, nothing asked of the desktop.
  await expectNothingFilled(page);
  expect(await page.evaluate(() => (window as unknown as { __seen: string[] }).__seen.filter((s) => s !== "|"))).toEqual([]);
  expect(await fake.log()).toEqual([]);

  // Control: the same page is filled by the shortcut, so the suite can tell a fill from a non-fill.
  const outcome = await fake.gesture({ kind: "shortcut", tabId: await fake.tabIdOf(BANK) });
  expect(outcome).toMatchObject({ ok: true });
  await expect(page.locator("#pass")).toHaveValue(PASSWORD);
  await expectFilled(page, BANK);
  expect((await fake.log()).map((e) => e.op)).toEqual(["hello", "listForOrigin", "fill"]);
});
