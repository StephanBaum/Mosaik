/**
 * The desktop's passkey rate windows belong to the pairing, but each site also has its own, a
 * smaller one (2 registrations and 6 sign-ins a minute). A page that spends its own windows with
 * forged requests the desktop refuses silently is refused `rateLimited` without costing the
 * pairing anything, so another site's registration and sign-in still reach the Mosaic dialog.
 */
import { BANK, EVIL, bankRp, expect, open, rpCreate, rpGet, test } from "../fixtures.ts";

interface Burner {
  burn(creates: number, gets: number): Promise<unknown[]>;
}

test("a page that spends its own passkey budget does not take another site's call away from Mosaic", async ({
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
  const burned = await fake.passkeyLog();
  expect(burned.every((entry) => entry.origin === EVIL)).toBe(true);
  // The site's own windows (2 and 6) ran out; the pairing's (3 and 10) did not.
  expect(burned.filter((entry) => entry.result === "rateLimited").map((entry) => entry.op)).toEqual([
    "passkeyCreate",
    ...Array.from({ length: 4 }, () => "passkeyGet"),
  ]);

  const bank = await context.newPage();
  await open(bank, bankRp);
  await bank.bringToFront();

  // The bank has its own windows: the desktop's dialog answers, not the browser's authenticator.
  await fake.passkeyDecide({ kind: "allow" });
  const created = await rpCreate(bank);
  expect(created.verify).toMatchObject({ ok: true });
  await fake.passkeyDecide({ kind: "allow" });
  expect(await rpGet(bank)).toMatchObject({ id: created.id, verify: { ok: true } });
  const bankLog = (await fake.passkeyLog()).filter((entry) => entry.origin === BANK);
  expect(bankLog.map((entry) => [entry.op, entry.result])).toEqual([
    ["passkeyCreate", "allow"],
    ["passkeyGet", "allow"],
  ]);
  await bank.close();
});
