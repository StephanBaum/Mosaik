/**
 * The guided password change (M7b, ADR-0068): the popup offers the change fill only for an open
 * change on the exact address, the content script classifies the form before any secret is
 * requested, one message carries the two values of one login, and nothing is ever submitted.
 * The pretend desktop decides like the real one (exact origin, https, frame rules); the hostile
 * cases (look-alikes, frames, spoofed results) are in e2e/hostile/rotation-*.
 */
import type { Page } from "@playwright/test";
import { expectAccessible } from "./axe.ts";
import { BANK, EVIL, PASSWORD, bankItem, expect, expectNothingFilled, inspect, open, openPopup, test, type Fake } from "./fixtures.ts";

const IDS = { vaultId: "v1", itemId: "i1", attemptId: "a0000000000000001.e2e.0" };
const NEW_VALUE = "Gen-Pw-7731!xYzQ-1";

const attempt = () => ({ ...IDS, value: NEW_VALUE, stage: "prepared" as const });
const config = (over: { begin?: "started" | "cancelled" } = {}) => ({
  items: [bankItem()],
  rotation: { attempts: [attempt()], begin: over.begin ?? ("started" as const), result: "confirmed" as const, addOrigin: "added" as const },
});

const change = (fake: Fake, tabId: number) =>
  fake.popupRequest(tabId, { op: "fillChange", ...IDS, confirmFrame: null });

const asked = async (fake: Fake, op: string): Promise<number> => (await fake.log()).filter((entry) => entry.op === op).length;

async function submitted(page: Page): Promise<boolean> {
  return page.evaluate(() => (window as unknown as { __submitted?: boolean }).__submitted === true);
}

test("the change fill puts the current password in the current field and the new one in both new fields, never the user name, and never submits", async ({ page, fake }) => {
  await fake.reset(config());
  await open(page, `${BANK}/change-password.html`);
  const tabId = await fake.tabIdOf(BANK);

  expect(await change(fake, tabId)).toEqual({ username: 0, password: 1, otp: 0, newPassword: 2 });
  const [frame] = await inspect(page);
  expect(frame?.values).toEqual({ user: "", current: PASSWORD, new: NEW_VALUE, confirm: NEW_VALUE });
  expect(await submitted(page)).toBe(false);
  expect((await fake.log()).filter((entry) => entry.op === "fillChange")).toEqual([
    { op: "fillChange", origin: BANK, topOrigin: BANK, frame: "top" },
  ]);
  // The desktop marked the attempt delivered: the card says so.
  expect((await fake.popupState(tabId)).rotations[0]).toMatchObject({ attemptId: IDS.attemptId, stage: "delivered" });
});

test("a page without a change form gets no secret requested", async ({ page, fake }) => {
  await fake.reset(config());
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);
  await expect(change(fake, tabId)).rejects.toThrow(/changeFormNotFound/);
  expect(await asked(fake, "fillChange")).toBe(0);
  await expectNothingFilled(page);
});

test("two password fields that cannot be told apart get no secret requested", async ({ page, fake }) => {
  await fake.reset(config());
  await open(page, `${BANK}/change-ambiguous.html`);
  const tabId = await fake.tabIdOf(BANK);
  await expect(change(fake, tabId)).rejects.toThrow(/changeFormAmbiguous/);
  expect(await asked(fake, "fillChange")).toBe(0);
  await expectNothingFilled(page);
});

test("a page that swaps the roles of its fields after the probe receives nothing", async ({ page, fake }) => {
  await fake.reset(config());
  await open(page, `${BANK}/change-swap.html`);
  const tabId = await fake.tabIdOf(BANK);

  await fake.pauseAfterProbe();
  const pending = change(fake, tabId).then(
    () => "filled",
    (error: Error) => error.message,
  );
  await expect.poll(() => fake.isPaused()).toBe(true);
  await page.evaluate(() => (window as unknown as { swap: () => void }).swap());
  await fake.release();

  expect(await pending).toMatch(/changeFormAmbiguous/);
  await expectNothingFilled(page);
});

test("the card is on the page of the login's own address only, and the new password is not in it", async ({ page, context, ext, fake }) => {
  await fake.reset(config());
  await open(page, `${BANK}/change-password.html`);
  const tabId = await fake.tabIdOf(BANK);
  const popup = await openPopup(context, ext.id, tabId);
  const card = popup.getByRole("region", { name: "Password change" });
  await expect(card).toBeVisible();
  await expect(card.getByRole("button", { name: "Fill change form" })).toBeVisible();
  await expect(popup.locator("body")).not.toContainText(NEW_VALUE);
  await expect(popup.locator("body")).not.toContainText(PASSWORD);
  await expectAccessible(popup);
  await popup.close();

  // A look-alike page: no card, and a forced request is refused by the desktop before it answers.
  await open(page, `${EVIL}/change-password.html`);
  const evilTab = await fake.tabIdOf(EVIL);
  expect((await fake.popupState(evilTab)).rotations).toEqual([]);
  await expect(change(fake, evilTab)).rejects.toThrow(/originMismatch/);
  await expectNothingFilled(page);
});

test("Fill change form in the popup fills the page, and It worked asks the desktop to confirm", async ({ page, context, ext, fake }) => {
  await fake.reset(config());
  await open(page, `${BANK}/change-password.html`);
  const tabId = await fake.tabIdOf(BANK);
  const popup = await openPopup(context, ext.id, tabId);

  await popup.getByRole("button", { name: "Fill change form" }).click();
  await expect(popup.getByRole("status").last()).toHaveText("Change form filled. Check it, then submit it yourself.");
  expect((await inspect(page))[0]?.values["new"]).toBe(NEW_VALUE);

  await popup.getByRole("button", { name: "It worked" }).click();
  await expect(popup.getByRole("status").last()).toHaveText("Done. The new password is now saved in Mosaic.");
  await expect(popup.getByRole("region", { name: "Password change" })).toHaveCount(0);
  expect((await fake.log()).filter((entry) => entry.op === "rotationResult")).toHaveLength(1);
  await popup.close();
});

test("Change password… starts a change through the desktop and then shows the card", async ({ page, context, ext, fake }) => {
  await fake.reset({ ...config(), rotation: { ...config().rotation, attempts: [] } });
  await open(page, `${BANK}/change-password.html`);
  const tabId = await fake.tabIdOf(BANK);
  const popup = await openPopup(context, ext.id, tabId);
  await expect(popup.getByRole("region", { name: "Password change" })).toHaveCount(0);
  await popup.getByRole("button", { name: /Change password… for/ }).click();
  await expect(popup.getByRole("region", { name: "Password change" })).toBeVisible();
  expect((await fake.log()).filter((entry) => entry.op === "rotationBegin")).toEqual([
    { op: "rotationBegin", origin: BANK, topOrigin: BANK, frame: null },
  ]);
  await popup.close();
});
