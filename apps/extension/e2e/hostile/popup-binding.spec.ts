/**
 * What the popup showed is what a click acts on: a frame confirmation names one document, and a
 * fill, a code fill, a generated password and a pending save stay with the tab's site. The page
 * tries to swap the frame or leave the site between the popup's state and the click.
 */
import type { Page } from "@playwright/test";
import { BANK, EVIL, bankItem, expect, expectNothingFilled, inspect, open, openPopup, test } from "../fixtures.ts";

async function framesLoaded(page: Page, marker: string): Promise<void> {
  await expect.poll(() => page.frames().some((f) => f.url().includes(marker))).toBe(true);
  for (const frame of page.frames()) {
    await frame.waitForLoadState("load");
  }
}

/** Replaces the frame's document (same frame id) or the frame element itself (a new frame id). */
async function swapFrame(page: Page, how: "reload" | "replace"): Promise<void> {
  await page.evaluate((mode) => {
    const old = document.getElementById("frame") as HTMLIFrameElement;
    if (mode === "reload") {
      old.src = "/login.html?swapped";
      return;
    }
    const fresh = document.createElement("iframe");
    Object.assign(fresh, { id: "frame", src: "/login.html?swapped", width: "400", height: "200" });
    old.replaceWith(fresh);
  }, how);
  await framesLoaded(page, "?swapped");
}

for (const how of ["reload", "replace"] as const) {
  for (const otpOnly of [false, true]) {
    test(`a confirmation is void once the page ${how}s the frame (${otpOnly ? "Fill code" : "Fill"})`, async ({ page, fake }) => {
      await fake.reset({ items: [bankItem({ totp: "123456" })] });
      await open(page, `${BANK}/same-origin-iframe.html`, 2);
      const tabId = await fake.tabIdOf(BANK);
      const shown = (await fake.popupState(tabId)).frameTarget;
      expect(shown).not.toBeNull();

      await swapFrame(page, how);
      expect(
        await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", otpOnly, confirmFrame: shown ?? false }),
      ).toEqual({ ok: false, error: "frameNeedsConfirmation" });
      expect(await fake.fills()).toEqual([]);
      await expectNothingFilled(page);
    });
  }
}

test("a confirmed frame that navigates to another site between probe and delivery gets nothing", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/same-origin-iframe.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  await fake.pauseAfterProbe();
  const pending = fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: true });
  await expect.poll(() => fake.isPaused()).toBe(true);
  await page.evaluate((evil) => {
    (document.getElementById("frame") as HTMLIFrameElement).src = `${evil}/login.html?swapped`;
  }, EVIL);
  await framesLoaded(page, "?swapped");
  await fake.release();

  expect(await pending).toMatchObject({ ok: false });
  await expectNothingFilled(page);
});

test("the popup's Fill after the tab moved to another site does not reach that site", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);
  expect((await fake.popupState(tabId)).matches).toHaveLength(1);

  // The popup listed the bank's login; before the click the page sends the tab elsewhere.
  await open(page, `${EVIL}/login.html`);
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1" })).toMatchObject({ ok: false });
  await expectNothingFilled(page);
  expect(await page.evaluate(() => (window as unknown as { __mosaicLeaks: unknown[] }).__mosaicLeaks.length)).toBe(0);
});

test("a generated password and its pending save do not survive leaving the site", async ({ page, fake }) => {
  await fake.reset({ items: [] });
  await open(page, `${BANK}/signup.html`);
  const tabId = await fake.tabIdOf(BANK);
  const forBank = await fake.popupGenerate(tabId);

  // Generated and shown on the bank; the tab moves to another site before Fill.
  await open(page, `${EVIL}/signup.html`);
  await expect(fake.popupRequest(tabId, { op: "fillGenerated", confirmFrame: null })).rejects.toThrow(/generatedExpired/);
  expect(JSON.stringify(await inspect(page))).not.toContain(forBank);

  // Back on the bank, the held password is gone (any take clears it).
  await open(page, `${BANK}/signup.html`);
  await expect(fake.popupRequest(tabId, { op: "fillGenerated", confirmFrame: null })).rejects.toThrow(/generatedExpired/);

  // A filled one waits as a pending save for the bank only, and is dropped when the tab leaves.
  await fake.popupGenerate(tabId);
  await fake.popupRequest(tabId, { op: "fillGenerated", confirmFrame: null });
  expect((await fake.popupState(tabId)).pendingSave).toMatchObject({ host: "bank.test", source: "generated" });
  await open(page, `${EVIL}/signup.html`);
  expect((await fake.popupState(tabId)).pendingSave).toBeNull();
  await open(page, `${BANK}/signup.html`);
  expect((await fake.popupState(tabId)).pendingSave).toBeNull();
});

test("the site's login page framed by another site cannot be filled from the popup, even when forced", async ({ page, context, ext, fake }) => {
  await fake.reset({ items: [bankItem({ totp: "123456" })] });
  await open(page, `${EVIL}/framed-by-evil.html`, 2);
  const tabId = await fake.tabIdOf(EVIL);
  const popup = await openPopup(context, ext.id, tabId);
  await expect(popup.getByText(/frame from another site/)).toBeVisible();
  // The frame's own logins are listed, but every Fill is off; Enter (the first match) is refused below too.
  const fills = popup.getByRole("button", { name: /^Fill/ });
  for (const button of await fills.all()) {
    await expect(button).toBeDisabled();
  }
  await popup.keyboard.press("Enter");
  await popup.close();

  for (const otpOnly of [false, true]) {
    expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", otpOnly, confirmFrame: true })).toEqual({
      ok: false,
      error: "crossOriginFrame",
    });
  }
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
});
