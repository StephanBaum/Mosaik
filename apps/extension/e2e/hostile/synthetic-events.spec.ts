import { BANK, bankItem, expect, expectNothingFilled, open, test } from "../fixtures.ts";

test("synthetic keys, clicks, context menus and messages from the page start nothing", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/synthetic-events.html`);
  // The page fires the shortcut, clicks, a context menu and fill-shaped messages every 200 ms.
  await page.waitForTimeout(2000);

  await expectNothingFilled(page);
  expect(await fake.log()).toEqual([]);
});
