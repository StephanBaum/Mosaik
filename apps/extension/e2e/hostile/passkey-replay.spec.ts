/**
 * A captured assertion is worth nothing at the relying party a second time (its challenge is
 * single use), and two sign-ins never share a challenge.
 */
import { bankRp, expect, open, rpCreate, rpGet, rpVerifyAssertion, test } from "../fixtures.ts";

test("a replayed assertion is refused and every sign-in has its own challenge", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, bankRp);
  await fake.passkeyDecide({ kind: "allow" });
  expect((await rpCreate(page)).verify).toMatchObject({ ok: true });

  await fake.passkeyDecide({ kind: "allow" });
  const first = await rpGet(page, { noVerify: true });
  await fake.passkeyDecide({ kind: "allow" });
  const second = await rpGet(page, { noVerify: true });
  expect(first.challenge).toBeDefined();
  expect(first.challenge).not.toBe(second.challenge);
  expect(first.signature).not.toBe(second.signature);

  expect(await rpVerifyAssertion(page, first)).toMatchObject({ status: 200, ok: true });
  expect(await rpVerifyAssertion(page, first)).toMatchObject({ status: 409, ok: false });
  expect(await rpVerifyAssertion(page, second)).toMatchObject({ status: 200, ok: true });
});
