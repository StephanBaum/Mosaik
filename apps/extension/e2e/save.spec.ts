/**
 * Save capture (ADR-0038): off by default; with the setting on, only a login the user submitted
 * (a trusted click, Enter) in the top frame or a same-origin frame is offered, and the desktop
 * confirms before anything is written (the fake desktop records what it was asked to save).
 */
import type { Page } from "@playwright/test";
import { expectAccessible } from "./axe.ts";
import { BANK, EVIL, expect, open, openPopup, test, type Fake } from "./fixtures.ts";

const USER = "ada@bank.test";
const PASSWORD = "hunter2-correct-horse";

async function submitLogin(page: Page, via: "click" | "enter" = "click"): Promise<void> {
  await page.locator("#user").fill(USER);
  await page.locator("#pass").fill(PASSWORD);
  if (via === "click") {
    await page.getByRole("button", { name: "Sign in" }).click();
  } else {
    await page.locator("#pass").press("Enter");
  }
}

const checks = async (fake: Fake): Promise<number> => (await fake.log()).filter((entry) => entry.op === "checkSave").length;

/** Gives the (asynchronous) capture a moment to arrive, then says how many checks the desktop saw. */
async function settledChecks(fake: Fake): Promise<number> {
  await new Promise((resolve) => setTimeout(resolve, 1200));
  return checks(fake);
}

test("capture is off by default: a submitted login is not looked at", async ({ page, fake }) => {
  await fake.reset({ items: [] });
  await open(page, `${BANK}/login.html`);
  await submitLogin(page);
  expect(await settledChecks(fake)).toBe(0);
  expect((await fake.popupState(await fake.tabIdOf(BANK))).pendingSave).toBeNull();
});

test("a submitted login is offered, saved by the desktop after a click, and forgotten", async ({ page, context, ext, fake }) => {
  await fake.reset({ items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);
  await submitLogin(page);
  await expect.poll(() => checks(fake)).toBe(1);
  expect((await fake.log()).find((entry) => entry.op === "checkSave")).toMatchObject({ origin: BANK, topOrigin: BANK });

  const popup = await openPopup(context, ext.id, tabId);
  const card = popup.getByRole("heading", { name: "Save this login for bank.test?" });
  await expect(card).toBeVisible();
  await expect(popup.getByText(USER)).toBeVisible();
  await expect(popup.locator("body")).not.toContainText(PASSWORD);
  expect(await fake.saves()).toEqual([]); // nothing is asked of the desktop before the click
  await expectAccessible(popup);

  await popup.getByRole("button", { name: "Save in Mosaic…" }).click();
  await expect(popup.getByRole("status").last()).toHaveText("Saved in Mosaic.");
  expect(await fake.saves()).toEqual([{ origin: BANK, topOrigin: BANK, username: USER, password: PASSWORD }]);
  expect((await fake.popupState(tabId)).pendingSave).toBeNull();
  await popup.close();
});

test("Enter in the password field offers the login too", async ({ page, fake }) => {
  await fake.reset({ items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/login.html`);
  await submitLogin(page, "enter");
  await expect.poll(() => checks(fake)).toBe(1);
  expect((await fake.popupState(await fake.tabIdOf(BANK))).pendingSave).toMatchObject({ host: "bank.test", username: USER });
});

for (const colorScheme of ["light", "dark"] as const) {
  test(`an update is named after the saved login it changes (${colorScheme})`, async ({ page, context, ext, fake }) => {
    await fake.reset({ items: [], save: { check: { kind: "update", itemTitle: "Bank" }, outcome: "updated" } });
    await fake.setCapture(true);
    await open(page, `${BANK}/login.html`);
    await submitLogin(page);
    await expect.poll(() => checks(fake)).toBe(1);
    const popup = await openPopup(context, ext.id, await fake.tabIdOf(BANK), { colorScheme });
    await expect(popup.getByRole("heading", { name: "Update the saved login for bank.test?" })).toBeVisible();
    await expect(popup.getByText("Updates “Bank”.")).toBeVisible();
    await expectAccessible(popup);
    await popup.getByRole("button", { name: "Save in Mosaic…" }).click();
    await expect(popup.getByRole("status").last()).toHaveText("Updated in Mosaic.");
    await popup.close();
  });
}

test("a login the desktop already has is not offered", async ({ page, fake }) => {
  await fake.reset({ items: [], save: { check: { kind: "unchanged", itemTitle: "Bank" }, outcome: "saved" } });
  await fake.setCapture(true);
  await open(page, `${BANK}/login.html`);
  await submitLogin(page);
  await expect.poll(() => checks(fake)).toBe(1);
  expect((await fake.popupState(await fake.tabIdOf(BANK))).pendingSave).toBeNull();
});

test("Not now forgets it, Never for this site stops the offers for the site", async ({ page, context, ext, fake }) => {
  await fake.reset({ items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);
  await submitLogin(page);
  await expect.poll(() => checks(fake)).toBe(1);

  let popup = await openPopup(context, ext.id, tabId);
  await popup.getByRole("button", { name: "Not now" }).click();
  await expect(popup.getByRole("heading", { name: "Save this login for bank.test?" })).toHaveCount(0);
  await popup.close();
  expect((await fake.popupState(tabId)).pendingSave).toBeNull();

  // A new submit (after the 2 s gap between captures) is offered again.
  await new Promise((resolve) => setTimeout(resolve, 2100));
  await submitLogin(page);
  await expect.poll(() => checks(fake)).toBe(2);
  popup = await openPopup(context, ext.id, tabId);
  await popup.getByRole("button", { name: "Never for this site" }).click();
  await expect(popup.getByRole("status").last()).toHaveText("Mosaic won't offer to save on bank.test again.");
  await popup.close();

  await new Promise((resolve) => setTimeout(resolve, 2100));
  await submitLogin(page);
  expect(await settledChecks(fake)).toBe(2);
  expect((await fake.popupState(tabId)).pendingSave).toBeNull();
  expect(await fake.saves()).toEqual([]);
});

test("a page cannot make the capture report a login: scripted events are ignored", async ({ page, fake }) => {
  await fake.reset({ items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/capture-synthetic.html`);
  await page.waitForFunction(() => (window as unknown as { __scripted?: boolean }).__scripted === true);
  expect(await settledChecks(fake)).toBe(0);
  expect((await fake.popupState(await fake.tabIdOf(BANK))).pendingSave).toBeNull();
});

test("a login typed into a frame of another site is not offered", async ({ page, fake }) => {
  await fake.reset({ items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/cross-origin-iframe.html`, 2);
  const frame = page.frameLocator("#evil");
  await frame.locator("#user").fill("victim");
  await frame.locator("#pass").fill("typed-into-the-frame");
  await frame.getByRole("button", { name: "Sign in" }).click();
  expect(await settledChecks(fake)).toBe(0);
  expect((await fake.popupState(await fake.tabIdOf(BANK))).pendingSave).toBeNull();
  expect(EVIL).not.toBe(BANK);
});

test("a login typed while the desktop is locked waits, and is asked about once it is unlocked", async ({ page, context, ext, fake }) => {
  await fake.reset({ connection: "locked", items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);
  await submitLogin(page);
  await expect.poll(async () => (await fake.popupState(tabId)).pendingSave?.kind ?? "unasked").toBe("unasked");
  expect(await checks(fake)).toBe(0);

  await fake.configure({ connection: "unlocked" });
  const popup = await openPopup(context, ext.id, tabId);
  await expect(popup.getByRole("heading", { name: "Save this login for bank.test?" })).toBeVisible();
  expect(await checks(fake)).toBe(1);
  await popup.close();
});

test("the More menu switch turns capture on and off", async ({ page, context, ext, fake }) => {
  await fake.reset({ items: [] });
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);
  const popup = await openPopup(context, ext.id, tabId);
  await popup.getByRole("button", { name: "More options" }).click();
  await expectAccessible(popup);
  await popup.getByRole("menuitemcheckbox", { name: /Offer to save logins/ }).click();
  await popup.getByRole("button", { name: "More options" }).click();
  await expect(popup.getByRole("menuitemcheckbox", { name: /Offer to save logins/ })).toHaveAttribute("aria-checked", "true");
  await popup.keyboard.press("Escape");

  await open(page, `${BANK}/login.html`);
  await submitLogin(page);
  await expect.poll(() => checks(fake)).toBe(1);

  await popup.getByRole("button", { name: "More options" }).click();
  await popup.getByRole("menuitemcheckbox", { name: /Offer to save logins/ }).click();
  await expect.poll(async () => (await fake.popupState(tabId)).saveOffers).toBe(false);
  // Turning it off forgot what was captured.
  expect((await fake.popupState(tabId)).pendingSave).toBeNull();
  await popup.close();
});
