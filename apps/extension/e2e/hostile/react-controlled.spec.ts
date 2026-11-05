import { BANK, PASSWORD, USERNAME, bankItem, expect, open, test } from "../fixtures.ts";

test("a controlled form whose onChange runs only for tracked changes receives the fill", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/react-controlled.html`);
  const tabId = await fake.tabIdOf(BANK);
  await expect(page.locator("#submit")).toBeDisabled();

  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true });

  // The page's state, not just the DOM, has the values: its onChange ran for both fields.
  await expect(page.locator("#submit")).toBeEnabled();
  expect(await page.evaluate(() => (window as unknown as { __state: unknown }).__state)).toMatchObject({
    userLength: USERNAME.length,
    passLength: PASSWORD.length,
    submitEnabled: true,
  });
  await expect(page.locator("#user")).toHaveValue(USERNAME);
  await expect(page.locator("#pass")).toHaveValue(PASSWORD);
});
