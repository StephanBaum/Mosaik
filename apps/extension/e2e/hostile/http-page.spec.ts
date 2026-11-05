import { BANK_HTTP, bankItem, expect, expectFilled, expectNothingFilled, open, test } from "../fixtures.ts";

test("the https login is not offered on the http version of the site", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK_HTTP}/login.html`);
  const tabId = await fake.tabIdOf(BANK_HTTP);

  const state = await fake.popupState(tabId);
  expect(state.tab).toMatchObject({ secure: false });
  expect(state.warnings).toContainEqual({ kind: "insecure" });
  expect(state.matches).toEqual([]);
  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "noMatch" });
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
});

test("an http login without the per-item override is blocked, with it the fill works", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem({ origin: BANK_HTTP, allowHttp: false })] });
  await open(page, `${BANK_HTTP}/login.html`);
  const tabId = await fake.tabIdOf(BANK_HTTP);

  const state = await fake.popupState(tabId);
  expect(state.matches).toMatchObject([{ itemId: "i1", blocked: "insecure" }]);
  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "noMatch" });
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1" })).toEqual({ ok: false, error: "insecureOrigin" });
  await expectNothingFilled(page);

  await fake.reset({ items: [bankItem({ origin: BANK_HTTP, allowHttp: true })] });
  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true });
  await expectFilled(page, BANK_HTTP);
});
