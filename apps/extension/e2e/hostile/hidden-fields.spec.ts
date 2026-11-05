import { BANK, USERNAME, PASSWORD, bankItem, expect, expectNothingFilled, inspect, open, test } from "../fixtures.ts";

test("only the visible username and password are filled; every hidden decoy stays empty", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/hidden-fields.html`);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({
    ok: true,
    summary: { username: 1, password: 1 },
  });
  const [frame] = await inspect(page);
  const filled = Object.entries(frame?.values ?? {}).filter(([, value]) => value !== "");
  expect(filled).toEqual([
    ["user", USERNAME],
    ["pass", PASSWORD],
  ]);
  // Not even an event with a value reached a decoy.
  expect((frame?.leaks ?? []).map((leak) => leak.id).sort()).toEqual(["pass", "pass", "user", "user"]);
});

test("a form that is entirely hidden is not filled, and the desktop is not asked", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/hidden-only.html`);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "noFields" });
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1" })).toEqual({ ok: false, error: "noFields" });
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
});
