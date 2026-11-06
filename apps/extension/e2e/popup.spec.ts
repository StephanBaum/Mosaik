/**
 * The popup in a tab of its own, about a hostile-site tab (`popup.html?e2eTabId=`), against the
 * fake desktop: the views of mockups E01, E02, E05 and E07 with their deviations (no master
 * password, no code shown, no "Fill anyway"), axe in light and dark, the frame confirmation
 * and the keyboard.
 */
import { expectAccessible } from "./axe.ts";
import {
  BANK,
  BANK_HTTP,
  bankItem,
  expect,
  expectFilled,
  expectNothingFilled,
  inspect,
  open,
  openPopup,
  test,
} from "./fixtures.ts";

const SCHEMES = ["light", "dark"] as const;

test.describe("views", () => {
  for (const colorScheme of SCHEMES) {
    test(`connect, locked and logins views are accessible (${colorScheme})`, async ({ page, context, ext, fake }) => {
      await open(page, `${BANK}/login.html`);
      const tabId = await fake.tabIdOf(BANK);

      await fake.reset({ connection: "notRunning", items: [bankItem()] });
      let popup = await openPopup(context, ext.id, tabId, { colorScheme });
      await expect(popup.getByRole("heading", { name: "Open Mosaic on this computer" })).toBeVisible();
      await expectAccessible(popup);
      await popup.close();

      await fake.reset({ paired: false, items: [bankItem()] });
      popup = await openPopup(context, ext.id, tabId, { colorScheme });
      await expect(popup.getByRole("heading", { name: "Connect to Mosaic" })).toBeVisible();
      await expectAccessible(popup);
      await popup.close();

      // E01 without a master password: the app unlocks.
      await fake.reset({ connection: "locked", items: [bankItem()] });
      popup = await openPopup(context, ext.id, tabId, { colorScheme });
      await expect(popup.getByRole("heading", { name: "Mosaic is locked" })).toBeVisible();
      await expect(popup.locator("input")).toHaveCount(0);
      await expectAccessible(popup);
      await popup.close();

      await fake.reset({ items: [bankItem(), bankItem({ itemId: "i2", title: "Bank (work)", username: "ada.work", totp: "123456" })] });
      popup = await openPopup(context, ext.id, tabId, { colorScheme });
      await expect(popup.getByRole("heading", { name: "For bank.test" })).toBeVisible();
      await expect(popup.getByRole("button", { name: "Fill ada@bank.test" })).toBeVisible();
      await expectAccessible(popup);
      // The copy menu, open.
      await popup.getByRole("button", { name: "Copy options for ada.work" }).click();
      await expect(popup.getByRole("menuitem", { name: /Copy code/ })).toBeVisible();
      await expectAccessible(popup);
      await popup.close();
    });
  }

  test("connecting pairs and shows the logins", async ({ page, context, ext, fake }) => {
    await open(page, `${BANK}/login.html`);
    const tabId = await fake.tabIdOf(BANK);
    await fake.reset({ paired: false, items: [bankItem()] });
    const popup = await openPopup(context, ext.id, tabId);
    await popup.getByRole("button", { name: "Connect" }).click();
    await expect(popup.getByRole("heading", { name: "For bank.test" })).toBeVisible();
    await popup.close();
  });

  test("speaks German", async ({ page, context, ext, fake }) => {
    await open(page, `${BANK}/login.html`);
    const tabId = await fake.tabIdOf(BANK);
    await fake.reset({ items: [bankItem()] });
    const popup = await openPopup(context, ext.id, tabId, { languages: ["de-DE", "de"] });
    await expect(popup.getByRole("heading", { name: "Für bank.test" })).toBeVisible();
    await expect(popup.getByRole("button", { name: "ada@bank.test ausfüllen" })).toBeVisible();
    await expectAccessible(popup);
    await popup.close();
  });
});

test.describe("filling from the popup", () => {
  test("a click on Fill fills the login, and the page gets only that login", async ({ page, context, ext, fake }) => {
    await fake.reset({ items: [bankItem()] });
    await open(page, `${BANK}/login.html`);
    const tabId = await fake.tabIdOf(BANK);
    const popup = await openPopup(context, ext.id, tabId);
    await expectNothingFilled(page);
    await popup.getByRole("button", { name: "Fill ada@bank.test" }).click();
    await expect(popup.getByRole("status").last()).toHaveText("Filled.");
    await expectFilled(page, `${BANK}/login.html`);
    expect(await fake.fills()).toEqual([{ op: "fill", origin: BANK, topOrigin: BANK, frame: "top" }]);
    await popup.close();
  });

  test("Enter fills the first match", async ({ page, context, ext, fake }) => {
    await fake.reset({ items: [bankItem()] });
    await open(page, `${BANK}/login.html`);
    const popup = await openPopup(context, ext.id, await fake.tabIdOf(BANK));
    await expect(popup.getByRole("button", { name: "Fill ada@bank.test" })).toBeVisible();
    await popup.keyboard.press("Enter");
    await expect(popup.getByRole("status").last()).toHaveText("Filled.");
    await expectFilled(page, `${BANK}/login.html`);
    await popup.close();
  });

  test("copies with Ctrl+C and says how long the clipboard keeps it", async ({ page, context, ext, fake }) => {
    await fake.reset({ items: [bankItem({ totp: "123456" })] });
    await open(page, `${BANK}/login.html`);
    const popup = await openPopup(context, ext.id, await fake.tabIdOf(BANK));
    await expect(popup.getByRole("button", { name: "Fill ada@bank.test" })).toBeVisible();
    await popup.keyboard.press("Control+c");
    await expect(popup.getByRole("status").last()).toHaveText("Copied. Clears in 30 seconds.");
    expect((await fake.log()).filter((entry) => entry.op === "copy")).toHaveLength(1);
    await popup.close();
  });

  test("shows the code step after a password fill, never the code, and fills it on a click", async ({ page, context, ext, fake }) => {
    await fake.reset({ items: [bankItem({ totp: "482913" })] });
    await open(page, `${BANK}/login.html`);
    const tabId = await fake.tabIdOf(BANK);
    // A password step: this page has no code field yet (a second step shows one later).
    await page.evaluate(() => document.getElementById("code")?.remove());
    await fake.popupState(tabId); // the popup lists the logins first, so the fill knows the item has a code
    expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1" })).toMatchObject({ ok: true });
    await page.evaluate(() => {
      const input = document.createElement("input");
      input.id = "code";
      input.autocomplete = "one-time-code";
      document.getElementById("login")?.append(input);
    });

    const popup = await openPopup(context, ext.id, tabId);
    await expect(popup.getByRole("heading", { name: "Fill your one-time code" })).toBeVisible();
    await expect(popup.locator("body")).not.toContainText("482913");
    await expectAccessible(popup);
    await popup.getByRole("button", { name: "Fill code" }).click();
    await expect(popup.getByRole("status").last()).toHaveText("Code filled.");
    const [frame] = await inspect(page);
    expect(frame?.values["code"]).toBe("482913");
    await popup.close();
  });
});

test.describe("warnings, without a way around them", () => {
  for (const colorScheme of SCHEMES) {
    test(`an international address and an unencrypted page (${colorScheme})`, async ({ page, context, ext, fake }) => {
      await fake.reset({ items: [bankItem()] });
      const host = new URL("https://bаnk.test").hostname;
      await open(page, `https://${host}:4843/login.html`);
      const tabId = await fake.tabIdOf(`https://${host}`);
      const popup = await openPopup(context, ext.id, tabId, { colorScheme });
      const alert = popup.getByRole("alert").first();
      await expect(alert).toContainText("Check the address");
      await expect(alert).toContainText("bаnk.test");
      await expect(alert).toContainText(host);
      await expect(popup.getByRole("button", { name: /fill/i })).toHaveCount(0);
      await expectAccessible(popup);
      await popup.close();

      await fake.reset({ items: [bankItem({ origin: BANK_HTTP, allowHttp: false })] });
      await open(page, `${BANK_HTTP}/login.html`);
      const httpTab = await fake.tabIdOf(BANK_HTTP);
      const httpPopup = await openPopup(context, ext.id, httpTab, { colorScheme });
      await expect(httpPopup.getByRole("alert")).toContainText("This page isn't encrypted");
      // The login does not allow plain http: its Fill is off.
      await expect(httpPopup.getByRole("button", { name: "Fill ada@bank.test" })).toBeDisabled();
      await expectAccessible(httpPopup);
      await httpPopup.close();
    });
  }
});

test.describe("frames", () => {
  test("the popup confirms the frame it showed, and the page gets the login after that", async ({ page, context, ext, fake }) => {
    await fake.reset({ items: [bankItem()] });
    await open(page, `${BANK}/same-origin-iframe.html`, 2);
    const tabId = await fake.tabIdOf(BANK);
    const popup = await openPopup(context, ext.id, tabId);
    await popup.getByRole("button", { name: "Fill ada@bank.test" }).click();
    const dialog = popup.getByRole("dialog");
    await expect(dialog).toContainText("Fill inside an embedded frame?");
    await expectAccessible(popup);
    expect(await fake.fills()).toEqual([]);
    await expectNothingFilled(page);
    await dialog.getByRole("button", { name: "Fill in the frame" }).click();
    await expect(popup.getByRole("status").last()).toHaveText("Filled.");
    await expectFilled(page, `${BANK}/login.html`);
    expect(await fake.fills()).toEqual([{ op: "fill", origin: BANK, topOrigin: BANK, frame: "sameOriginFrame" }]);
    await popup.close();
  });

  test("a confirmation of another frame or document is refused by the background", async ({ page, fake }) => {
    await fake.reset({ items: [bankItem()] });
    await open(page, `${BANK}/same-origin-iframe.html`, 2);
    const tabId = await fake.tabIdOf(BANK);
    const shown = (await fake.popupState(tabId)).frameTarget;
    expect(shown).not.toBeNull();
    const frameId = shown?.frameId ?? 0;
    for (const confirmFrame of [
      { frameId, documentId: "someone-elses-document" },
      { frameId: frameId + 1, documentId: shown?.documentId ?? null },
      { frameId: 0, documentId: null },
    ]) {
      expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame })).toEqual({
        ok: false,
        error: "frameNeedsConfirmation",
      });
    }
    expect(await fake.fills()).toEqual([]);
    await expectNothingFilled(page);
    // The frame the popup showed is accepted.
    expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: shown ?? true })).toMatchObject({ ok: true });
  });

  test("a frame of another site cannot be filled from the popup", async ({ page, context, ext, fake }) => {
    await fake.reset({ items: [bankItem()] });
    await open(page, `${BANK}/cross-origin-only.html`, 2);
    const tabId = await fake.tabIdOf(BANK);
    const popup = await openPopup(context, ext.id, tabId);
    await expect(popup.getByText(/frame from another site/)).toBeVisible();
    // The frame's origin is not the site's: nothing is listed for it, so there is nothing to click.
    await expect(popup.getByRole("button", { name: /^Fill/ })).toHaveCount(0);
    expect(await fake.fills()).toEqual([]);
    await popup.close();
  });
});
