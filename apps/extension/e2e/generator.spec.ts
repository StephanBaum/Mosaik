/**
 * The generator view (E03) against a sign-up page: the shown password is the one that is
 * filled, it is bound to its tab and origin and gone after one use, and filling it offers to
 * save it (confirmed in the desktop, never by the extension alone).
 */
import { expectAccessible } from "./axe.ts";
import { BANK, EVIL, expect, inspect, open, openPopup, test } from "./fixtures.ts";

const GENERATED = /^Gen-Pw-7731!xYzQ-\d+$/;

test("the password shown is the password filled, and it is offered for saving once", async ({ page, context, ext, fake }) => {
  await fake.reset({ items: [] });
  await open(page, `${BANK}/signup.html`);
  await page.locator("#new").click();
  const tabId = await fake.tabIdOf(BANK);

  const popup = await openPopup(context, ext.id, tabId);
  await popup.getByRole("button", { name: "Password generator" }).click();
  const output = popup.getByLabel("Generated password");
  await expect(output).toHaveText(GENERATED);
  await expect(output).toHaveAttribute("data-secret", "");
  await expect(popup.getByRole("heading", { name: "Generate a password for bank.test" })).toBeVisible();
  const shown = (await output.textContent()) ?? "";

  await popup.getByRole("button", { name: "Fill", exact: true }).click();
  await expect(popup.getByRole("status").last()).toHaveText("Filled. Mosaic will offer to save it.");
  const [frame] = await inspect(page);
  expect(frame?.values).toMatchObject({ new: shown, confirm: shown });

  // Used once: the background holds nothing more, so another fill is refused instead of making up a password.
  await expect(fake.popupRequest(tabId, { op: "fillGenerated", confirmFrame: null })).rejects.toThrow(/generatedExpired/);

  // The save prompt is the same E04 card; the desktop decides.
  expect((await fake.popupState(tabId)).pendingSave).toMatchObject({ host: "bank.test", source: "generated", kind: "new" });
  await popup.close();
  const second = await openPopup(context, ext.id, tabId);
  await expect(second.getByRole("heading", { name: "Save this login for bank.test?" })).toBeVisible();
  await expect(second.getByText("A password Mosaic generated for bank.test.")).toBeVisible();
  await second.getByRole("button", { name: "Save in Mosaic…" }).click();
  await expect(second.getByRole("status").last()).toHaveText("Saved in Mosaic.");
  expect(await fake.saves()).toEqual([{ origin: BANK, topOrigin: BANK, username: null, password: shown }]);
  expect((await fake.popupState(tabId)).pendingSave).toBeNull();
  await second.close();
});

test("a password generated for one site is not filled into another tab", async ({ page, context, fake }) => {
  await fake.reset({ items: [] });
  await open(page, `${BANK}/signup.html`);
  const other = await context.newPage();
  try {
    await open(other, `${EVIL}/signup.html`);
    await other.locator("#new").click();
    const bankTab = await fake.tabIdOf(BANK);
    const evilTab = await fake.tabIdOf(EVIL);
    const forBank = await fake.popupGenerate(bankTab);
    await expect(fake.popupRequest(evilTab, { op: "fillGenerated", confirmFrame: null })).rejects.toThrow(/generatedExpired/);
    expect(JSON.stringify(await inspect(other))).not.toContain(forBank);
    // The hold is gone for the site it was meant for, too: one take, then nothing.
    await expect(fake.popupRequest(bankTab, { op: "fillGenerated", confirmFrame: null })).rejects.toThrow(/generatedExpired/);
  } finally {
    await other.close();
  }
});

test("Fill is off without a new-password field in focus, and Copy goes through the desktop", async ({ page, context, ext, fake }) => {
  await fake.reset({ items: [] });
  await open(page, `${BANK}/login.html`);
  const popup = await openPopup(context, ext.id, await fake.tabIdOf(BANK));
  await popup.getByRole("button", { name: "Password generator" }).click();
  await expect(popup.getByLabel("Generated password")).toHaveText(GENERATED);
  await expect(popup.getByRole("button", { name: "Fill", exact: true })).toBeDisabled();
  await expect(popup.getByText(/click into a new-password field/)).toBeVisible();
  await popup.getByRole("button", { name: "Copy" }).click();
  await expect(popup.getByRole("status").last()).toHaveText("Copied. Clears in 30 seconds.");
  expect((await fake.log()).filter((entry) => entry.op === "copy")).toHaveLength(1);
  await popup.close();
});

for (const colorScheme of ["light", "dark"] as const) {
  test(`the generator is accessible (${colorScheme})`, async ({ page, context, ext, fake }) => {
    await fake.reset({ items: [] });
    await open(page, `${BANK}/signup.html`);
    await page.locator("#new").click();
    const popup = await openPopup(context, ext.id, await fake.tabIdOf(BANK), { colorScheme });
    await popup.getByRole("button", { name: "Password generator" }).click();
    await expect(popup.getByLabel("Generated password")).toHaveText(GENERATED);
    await expectAccessible(popup);
    await popup.getByRole("radio", { name: "Passphrase" }).click();
    await expect(popup.getByRole("slider", { name: "Words" })).toBeVisible();
    await expectAccessible(popup);
    await popup.close();
  });
}
