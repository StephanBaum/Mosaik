import { BANK, bankItem, expect, expectFilled, expectNothingFilled, open, test } from "../fixtures.ts";

test("an about:blank frame of the site counts as a same-origin frame: no fill without confirmation", async ({
  page,
  fake,
}) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/about-blank-frame.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  // The browser injects into the frame (it inherits the site's origin), so it is a frame like any other.
  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "frameNeedsConfirmation" });
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
  expect((await fake.popupState(tabId)).frame).toBe("sameOriginFrame");

  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: true })).toMatchObject({ ok: true });
  await expectFilled(page, "about:blank");
  expect(await fake.fills()).toEqual([{ op: "fill", origin: BANK, topOrigin: BANK, frame: "sameOriginFrame" }]);
});

test("an about:blank frame inside another origin's frame is never filled", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/about-blank-in-evil.html`, 3);
  const tabId = await fake.tabIdOf(BANK);

  // It inherits the other origin: a cross-origin frame, even when confirmed.
  for (const gesture of [
    { kind: "shortcut", tabId },
    { kind: "popupFill", tabId, itemId: "i1", confirmFrame: true },
  ] as const) {
    expect(await fake.gesture(gesture)).toEqual({ ok: false, error: "crossOriginFrame" });
  }
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
});
