/**
 * A registration for a credential Mosaic already holds ends in InvalidStateError, and aborting
 * a call (AbortController) closes the desktop dialog of exactly that request.
 */
import { bankRp, expect, open, pendingDialog, rpAbort, rpCreate, rpResult, rpStart, test } from "../fixtures.ts";

test("excludeCredentials that hit end with InvalidStateError after the desktop says so", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, bankRp);
  await fake.passkeyDecide({ kind: "allow" });
  const first = await rpCreate(page);
  expect(first.verify).toMatchObject({ ok: true });

  await fake.passkeyDecide({ kind: "acknowledge" });
  const again = await rpCreate(page, { exclude: [first.id ?? ""], noVerify: true });
  expect(again.error?.name).toBe("InvalidStateError");
  expect((await fake.passkeyLog()).map((e) => e.result)).toEqual(["allow", "passkeyExcluded"]);
});

test("aborting a call cancels the desktop dialog of that request", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, bankRp);
  await rpStart(page, "create", { noVerify: true });
  const dialog = await pendingDialog(fake);
  await rpAbort(page);
  expect((await rpResult(page)).error?.name).toBe("AbortError");
  await expect
    .poll(async () => (await fake.passkeyLog()).filter((e) => e.op === "passkeyCancel").map((e) => e.requestId))
    .toEqual([dialog.requestId]);
  expect(await fake.passkeyPending()).toEqual([]);
});

test("a signal that is already aborted never reaches the desktop", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, bankRp);
  const name = await page.evaluate(async () => {
    const controller = new AbortController();
    controller.abort();
    const rp = (window as unknown as { rp: { create(o: object): Promise<{ error?: { name: string } }> } }).rp;
    return (await rp.create({ signal: controller.signal, noVerify: true })).error?.name;
  });
  expect(name).toBe("AbortError");
  expect(await fake.passkeyLog()).toEqual([]);
});
