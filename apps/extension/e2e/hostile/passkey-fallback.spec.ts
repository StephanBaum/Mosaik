/**
 * What Mosaic does not answer goes to the browser's own WebAuthn, with the page's own options:
 * the dialog's "Use another device", a locked or unpaired desktop, and the setting off. A CDP
 * virtual authenticator stands for the browser's authenticator, so the fallback is observable.
 */
import type { BrowserContext, Page } from "@playwright/test";
import { addVirtualAuthenticator, bankRp, expect, open, rpCreate, rpGet, test, type Fake } from "../fixtures.ts";

/** Opens bank.test's page with a virtual authenticator, with the passkey setting on. */
async function start(page: Page, context: BrowserContext, fake: Fake, config: Parameters<Fake["reset"]>[0] = {}) {
  await fake.reset(config);
  await fake.setPasskeys(true);
  await open(page, bankRp);
  return addVirtualAuthenticator(context, page);
}

test('"Use another device" hands the call to the browser, which makes the credential', async ({ page, context, fake }) => {
  const authenticator = await start(page, context, fake);
  await fake.passkeyDecide({ kind: "useAnotherDevice" });
  const created = await rpCreate(page);
  expect(created.verify).toMatchObject({ ok: true });
  expect(await authenticator.credentials()).toMatchObject([{ credentialId: created.id, rpId: "bank.test" }]);
  expect(await fake.passkeyLog()).toMatchObject([{ op: "passkeyCreate", result: "useAnotherDevice" }]);
});

test("a locked desktop is not asked; the browser answers", async ({ page, context, fake }) => {
  const authenticator = await start(page, context, fake, { connection: "locked" });
  expect((await rpCreate(page)).verify).toMatchObject({ ok: true });
  expect(await authenticator.credentials()).toHaveLength(1);
  expect(await fake.passkeyLog()).toEqual([]);
});

test("a browser that is not paired is not asked; the browser answers", async ({ page, context, fake }) => {
  const authenticator = await start(page, context, fake, { paired: false });
  expect((await rpCreate(page)).verify).toMatchObject({ ok: true });
  expect(await authenticator.credentials()).toHaveLength(1);
  expect(await fake.passkeyLog()).toEqual([]);
});

test("a site with no passkey of Mosaic's is answered without a dialog, by the browser", async ({ page, context, fake }) => {
  const authenticator = await start(page, context, fake);
  await fake.passkeyDecide({ kind: "useAnotherDevice" });
  const created = await rpCreate(page);
  // The desktop holds nothing for bank.test: it says so without a dialog, and the browser signs in.
  const got = await rpGet(page);
  expect(got).toMatchObject({ id: created.id, verify: { ok: true } });
  expect((await fake.passkeyLog()).map((e) => e.result)).toEqual(["useAnotherDevice", "passkeyNotFound"]);
  expect(await authenticator.credentials()).toHaveLength(1);
});

test("with the setting off nothing is replaced, and a page that still has the shim falls back", async ({ page, context, fake }) => {
  await fake.reset();
  await open(page, bankRp);
  expect(await page.evaluate(() => navigator.credentials.create.toString().includes("[native code]"))).toBe(true);

  // Turned off while a page already runs the shim: the background answers "fallback" and the browser takes over.
  await fake.setPasskeys(true);
  await page.reload();
  expect(await page.evaluate(() => navigator.credentials.create.toString().includes("[native code]"))).toBe(false);
  const authenticator = await addVirtualAuthenticator(context, page);
  await fake.setPasskeys(false);
  expect((await rpCreate(page)).verify).toMatchObject({ ok: true });
  expect(await authenticator.credentials()).toHaveLength(1);
  expect(await fake.passkeyLog()).toEqual([]);
});
