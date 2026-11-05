import { BANK, bankItem, expect, expectFilled, expectNothingFilled, open, test } from "../fixtures.ts";

test("a same-origin frame is filled only after the user confirms it in the popup", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/same-origin-iframe.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  // The shortcut and an unconfirmed popup click both stop at the confirmation.
  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "frameNeedsConfirmation" });
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1" })).toEqual({
    ok: false,
    error: "frameNeedsConfirmation",
  });
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
  expect((await fake.popupState(tabId)).frame).toBe("sameOriginFrame");

  // Confirmed: the frame is filled, and the desktop is told it is a same-origin frame.
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: true })).toMatchObject({ ok: true });
  await expectFilled(page, `${BANK}/login.html`);
  expect(await fake.fills()).toEqual([{ op: "fill", origin: BANK, topOrigin: BANK, frame: "sameOriginFrame" }]);
});
