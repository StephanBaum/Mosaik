/**
 * evil.test asks for a passkey of bank.test (rpId "bank.test"). The desktop is told the real
 * origin and refuses the rp id, so the request is not answered by Mosaic; the browser's own
 * WebAuthn then refuses it too (SecurityError). No credential of bank.test comes into being.
 */
import { EVIL, addVirtualAuthenticator, evilRp, expect, open, rpCreate, rpGet, test } from "../fixtures.ts";

test("a page cannot get a passkey for another site's rp id", async ({ page, context, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, evilRp);
  const authenticator = await addVirtualAuthenticator(context, page);

  const created = await rpCreate(page, { rpId: "bank.test", noVerify: true });
  expect(created.error?.name).toBe("SecurityError");
  const got = await rpGet(page, { rpId: "bank.test", noVerify: true });
  expect(got.error?.name).toBe("SecurityError");

  const log = await fake.passkeyLog();
  expect(log).toMatchObject([
    { op: "passkeyCreate", origin: EVIL, topOrigin: EVIL, rpId: "bank.test", result: "rpIdMismatch" },
    { op: "passkeyGet", origin: EVIL, topOrigin: EVIL, rpId: "bank.test", result: "rpIdMismatch" },
  ]);
  expect(await fake.passkeyPending()).toEqual([]);
  expect(await authenticator.credentials()).toEqual([]);
});
