/**
 * The popup's More-menu switch "Use Mosaic for passkeys" (ADR-0040): off by default, accessible,
 * registers the passkey scripts when turned on and removes them when turned off.
 */
import { expectAccessible } from "./axe.ts";
import { bankRp, expect, open, openPopup, test } from "./fixtures.ts";

for (const colorScheme of ["light", "dark"] as const) {
  test(`the More menu switch turns passkeys on and off (${colorScheme})`, async ({ page, context, ext, fake }) => {
    await fake.reset();
    await open(page, bankRp);
    const tabId = await fake.tabIdOf(bankRp);
    expect((await fake.popupState(tabId)).passkeys).toBe(false);

    const popup = await openPopup(context, ext.id, tabId, { colorScheme });
    await popup.getByRole("button", { name: "More options" }).click();
    const item = popup.getByRole("menuitemcheckbox", { name: /Use Mosaic for passkeys/ });
    await expect(item).toHaveAttribute("aria-checked", "false");
    await expect(popup.getByText("Needs access to all websites to answer passkey requests. You confirm each one in the Mosaic app.")).toBeVisible();
    await expectAccessible(popup);

    await item.click();
    await expect.poll(async () => (await fake.popupState(tabId)).passkeys).toBe(true);
    await popup.getByRole("button", { name: "More options" }).click();
    await expect(popup.getByRole("menuitemcheckbox", { name: /Use Mosaic for passkeys/ })).toHaveAttribute("aria-checked", "true");
    await popup.keyboard.press("Escape");

    // A page opened now has the shim; one opened after turning it off does not.
    await open(page, bankRp);
    expect(await page.evaluate(() => navigator.credentials.get.toString().includes("[native code]"))).toBe(false);

    await popup.getByRole("button", { name: "More options" }).click();
    await popup.getByRole("menuitemcheckbox", { name: /Use Mosaic for passkeys/ }).click();
    await expect.poll(async () => (await fake.popupState(tabId)).passkeys).toBe(false);
    await open(page, bankRp);
    expect(await page.evaluate(() => navigator.credentials.get.toString().includes("[native code]"))).toBe(true);
    await popup.close();
  });
}

test("the switch speaks German", async ({ page, context, ext, fake }) => {
  await fake.reset();
  await open(page, bankRp);
  const popup = await openPopup(context, ext.id, await fake.tabIdOf(bankRp), { languages: ["de-DE", "de"] });
  await popup.getByRole("button", { name: "Weitere Optionen" }).click();
  await expect(popup.getByRole("menuitemcheckbox", { name: /Mosaic für Passkeys verwenden/ })).toBeVisible();
  await popup.close();
});
