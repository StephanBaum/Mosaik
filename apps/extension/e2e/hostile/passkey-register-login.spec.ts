/**
 * Passkeys on (ADR-0040): a registration and a sign-in on bank.test go through the shim, the
 * bridge, the background and the (fake) desktop dialog, and the relying party's independent
 * verifier accepts both. The origin the desktop was told is the browser's, not the page's.
 */
import { BANK, FLAGS_CREATE_UV, FLAGS_GET_UV, bankRp, expect, open, rpCreate, rpGet, test } from "../fixtures.ts";

test("a passkey is created and used with the desktop's approval and verifies at the server", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, bankRp);
  expect(await page.evaluate(() => navigator.credentials.create.toString().includes("[native code]"))).toBe(false);

  await fake.passkeyDecide({ kind: "allow" });
  const created = await rpCreate(page);
  expect(created).toMatchObject({
    type: "public-key",
    attachment: "platform",
    isPublicKeyCredential: true,
    extensions: { credProps: { rk: true } },
    verify: { status: 200, ok: true, flags: FLAGS_CREATE_UV, signCount: 0, credentialId: created.id },
  });
  const clientData = JSON.parse(Buffer.from(created.clientDataJSON ?? "", "base64url").toString("utf8")) as Record<string, unknown>;
  expect(clientData).toMatchObject({ type: "webauthn.create", origin: BANK, crossOrigin: false });

  await fake.passkeyDecide({ kind: "allow" });
  const signedIn = await rpGet(page);
  expect(signedIn).toMatchObject({
    id: created.id,
    isPublicKeyCredential: true,
    verify: { status: 200, ok: true, flags: FLAGS_GET_UV, signCount: 0, credentialId: created.id },
  });
  expect(signedIn.userHandle).toBe("AQIDBAUGBwgJCgsMDQ4PEA");

  // The desktop saw the browser's origin (twice the same, for the top frame) and request ids made in the background.
  const log = await fake.passkeyLog();
  expect(log.map((e) => e.op)).toEqual(["passkeyCreate", "passkeyGet"]);
  for (const entry of log) {
    expect(entry).toMatchObject({ origin: BANK, topOrigin: BANK, result: "allow" });
    expect(entry.requestId).toMatch(/^[0-9a-f]{32}$/);
  }
  expect(log[0]?.requestId).not.toBe(log[1]?.requestId);
});

test("nothing happens without the setting: the browser's own WebAuthn is untouched", async ({ page, fake }) => {
  await fake.reset();
  await open(page, bankRp);
  expect(await page.evaluate(() => navigator.credentials.create.toString().includes("[native code]"))).toBe(true);
  expect(await page.evaluate(() => navigator.credentials.get.toString().includes("[native code]"))).toBe(true);
  expect(await fake.passkeyLog()).toEqual([]);
});
