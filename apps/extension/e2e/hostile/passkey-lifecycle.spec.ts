/**
 * A desktop dialog must not outlive the call it belongs to: not when the page aborts before the
 * background has asked the desktop, not when the shim has already handed a slow page to the
 * browser, and not when the tab moves on. An approval in a dialog nobody waits for would sign
 * for nobody or store a passkey the site never received.
 */
import type { Page } from "@playwright/test";
import {
  BANK,
  addVirtualAuthenticator,
  bankRp,
  evilRp,
  expect,
  open,
  pendingDialog,
  rpCreate,
  rpStart,
  test,
  type Fake,
} from "../fixtures.ts";

const RACE = `${BANK}/passkey/abort-race.html`;

interface RacePage {
  race: {
    abortAtOnce(op: "create" | "get"): Promise<{ ok: boolean; name?: string }>;
    blockAfter(op: "create" | "get", ms: number): Promise<{ ok: boolean; id?: string; name?: string }>;
  };
}

/** No dialog is open now, and none stayed open for a moment after the call was over. */
async function noDialogLeft(page: Page, fake: Fake): Promise<void> {
  await page.waitForTimeout(800);
  expect(await fake.passkeyPending()).toEqual([]);
  expect((await fake.passkeyLog()).filter((entry) => entry.result === "opened")).toEqual([]);
}

test("a call aborted in the task that started it leaves no desktop dialog open", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, RACE);
  // A passkey of Mosaic's for bank.test, so that a sign-in would open a dialog.
  await fake.passkeyDecide({ kind: "allow" });
  expect((await rpCreate(page, { noVerify: true })).id).toBeTruthy();

  for (const op of ["create", "get"] as const) {
    expect(await page.evaluate((o) => (window as unknown as RacePage).race.abortAtOnce(o), op)).toEqual({
      ok: false,
      name: "AbortError",
    });
    await noDialogLeft(page, fake);
  }
});

test("a page too busy to hear the bridge in time gets the browser, and Mosaic's dialog does not stay open", async ({
  page,
  context,
  fake,
}) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, RACE);
  const authenticator = await addVirtualAuthenticator(context, page);
  // After the busy loop two queued tasks race: the shim's 1 s timer (it falls back to the browser and
  // cancels the bridged request) and the bridge's acknowledgement (Mosaic keeps the call and a
  // dialog opens). Either is correct, never both. A queued "use another device" answers the dialog
  // in the second case, so both end with exactly one credential from the browser's authenticator.
  await fake.passkeyDecide({ kind: "useAnotherDevice" });
  const result = await page.evaluate(() => (window as unknown as RacePage).race.blockAfter("create", 1500));
  expect(result).toMatchObject({ ok: true });
  expect(await authenticator.credentials()).toHaveLength(1);
  await noDialogLeft(page, fake);
});

test("a tab that loads another page or moves to another site cancels the dialog, and the new page hears nothing", async ({
  page,
  fake,
}) => {
  await fake.reset();
  await fake.setPasskeys(true);
  for (const next of [`${bankRp}?again`, evilRp]) {
    await open(page, bankRp);
    await rpStart(page, "create", { noVerify: true });
    const dialog = await pendingDialog(fake);
    await open(page, next);
    await page.evaluate(() => {
      const heard: unknown[] = [];
      (window as unknown as { heard: unknown[] }).heard = heard;
      window.addEventListener("message", (event) => {
        heard.push(event.data);
      });
    });
    await expect
      .poll(async () => (await fake.passkeyLog()).filter((e) => e.op === "passkeyCancel").map((e) => e.requestId))
      .toContain(dialog.requestId);
    expect(await fake.passkeyPending()).toEqual([]);
    await page.waitForTimeout(400);
    expect(await page.evaluate(() => (window as unknown as { heard: unknown[] }).heard)).toEqual([]);
  }
});
