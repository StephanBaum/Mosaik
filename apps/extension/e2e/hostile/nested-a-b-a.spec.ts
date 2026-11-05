import { BANK, bankItem, expect, expectNothingFilled, open, test } from "../fixtures.ts";

test("our own login form inside a hostile frame inside our page is never filled, even confirmed", async ({
  page,
  fake,
}) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/nested-a-b-a.html`, 3);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "crossOriginFrame" });
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: true })).toEqual({
    ok: false,
    error: "crossOriginFrame",
  });
  expect((await fake.popupState(tabId)).frame).toBe("crossOriginFrame");
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
});
