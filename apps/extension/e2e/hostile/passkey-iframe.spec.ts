/**
 * Subframes get no passkey scripts: a same-origin and a cross-origin frame of bank.test call
 * WebAuthn natively and the desktop is never asked.
 */
import { addVirtualAuthenticator, expect, open, rpCreate, rpGet, test, BANK } from "../fixtures.ts";

test("frames reach the browser's authenticator, never Mosaic", async ({ page, context, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, `${BANK}/passkey/frames.html`, 3);
  const authenticator = await addVirtualAuthenticator(context, page);
  const same = page.frames().find((f) => f.url().endsWith("/passkey/rp.html") && f.url().startsWith(BANK));
  const other = page.frames().find((f) => f.url().includes("evil.test"));
  expect(same).toBeDefined();
  expect(other).toBeDefined();

  // The top frame has the shim, the frames do not.
  expect(await page.evaluate(() => navigator.credentials.get.toString().includes("[native code]"))).toBe(false);
  for (const frame of [same, other]) {
    expect(await frame?.evaluate(() => navigator.credentials.get.toString().includes("[native code]"))).toBe(true);
  }

  // The same-origin frame registers and signs in with the virtual authenticator, verified by the server.
  if (same === undefined || other === undefined) {
    return;
  }
  const created = await rpCreate(same);
  expect(created.verify).toMatchObject({ ok: true });
  expect(await rpGet(same)).toMatchObject({ verify: { ok: true } });
  expect((await authenticator.credentials()).map((c) => c.rpId)).toEqual(["bank.test"]);

  // The cross-origin frame has no credential for its own site and is refused natively.
  expect((await rpGet(other, { noVerify: true })).error?.name).toBeDefined();

  expect(await fake.passkeyLog()).toEqual([]);
});
