/**
 * The desktop's passkey rate windows belong to the pairing, so every site shares them. A page
 * that spends them with forged requests the desktop refuses silently must not turn another
 * site's registration or sign-in into an error: a rate-limited request goes to the browser's own
 * WebAuthn like every other request Mosaic does not answer.
 */
import {
  BANK,
  EVIL,
  addVirtualAuthenticator,
  bankRp,
  expect,
  open,
  rpCreate,
  rpGet,
  test,
} from "../fixtures.ts";

interface Burner {
  burn(creates: number, gets: number): Promise<unknown[]>;
}

test("a page that spends the desktop's passkey budget does not take another site's call away from the browser", async ({
  page,
  context,
  fake,
}) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, `${EVIL}/passkey/burn-budget.html`);
  const answers = await page.evaluate(() => (window as unknown as Burner).burn(3, 10));
  // Every forged request was refused without a dialog.
  expect(answers).toEqual(Array.from({ length: 13 }, () => ({ kind: "fallback" })));
  expect(await fake.passkeyPending()).toEqual([]);
  expect((await fake.passkeyLog()).every((entry) => entry.origin === EVIL)).toBe(true);

  const bank = await context.newPage();
  await open(bank, bankRp);
  await bank.bringToFront();
  const authenticator = await addVirtualAuthenticator(context, bank);

  // The desktop says rateLimited; the browser's own authenticator registers and signs in.
  const created = await rpCreate(bank);
  expect(created.verify).toMatchObject({ ok: true });
  expect(await authenticator.credentials()).toMatchObject([{ credentialId: created.id, rpId: "bank.test" }]);
  expect(await rpGet(bank, { allow: [created.id ?? ""] })).toMatchObject({ id: created.id, verify: { ok: true } });
  const bankLog = (await fake.passkeyLog()).filter((entry) => entry.origin === BANK);
  expect(bankLog.map((entry) => [entry.op, entry.result])).toEqual([
    ["passkeyCreate", "rateLimited"],
    ["passkeyGet", "rateLimited"],
  ]);
  await bank.close();
});
