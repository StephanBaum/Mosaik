import { BANK, EVIL, bankItem, expect, expectNothingFilled, open, test } from "../fixtures.ts";

test("a tab that navigates to another origin after the probe receives nothing", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);

  await fake.pauseAfterProbe();
  const pending = fake.gesture({ kind: "popupFill", tabId, itemId: "i1" });
  await expect.poll(() => fake.isPaused()).toBe(true);

  // The page the user clicked on is replaced by another origin's login form.
  await page.goto(`${EVIL}/login.html`);
  await fake.release();

  const outcome = await pending;
  expect(outcome).toMatchObject({ ok: false });
  await expectNothingFilled(page);
  expect(page.url().startsWith(EVIL)).toBe(true);
});
