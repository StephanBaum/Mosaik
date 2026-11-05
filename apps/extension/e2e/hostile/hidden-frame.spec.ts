import { BANK, bankItem, expect, expectFilled, expectNothingFilled, open, test } from "../fixtures.ts";

for (const mode of ["offscreen", "covered", "filter", "below"] as const) {
  test(`a same-origin frame hidden by "${mode}" is not filled, even when the user confirms`, async ({ page, fake }) => {
    await fake.reset({ items: [bankItem()] });
    await open(page, `${BANK}/hidden-frame.html?mode=${mode}`, 2);
    const tabId = await fake.tabIdOf(BANK);

    expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: true })).toEqual({
      ok: false,
      error: "crossOriginFrame",
    });
    expect((await fake.popupState(tabId)).frame).toBe("crossOriginFrame");
    expect(await fake.fills()).toEqual([]);
    await expectNothingFilled(page);
  });
}

test("control: the same frame, visible, is filled after the confirmation", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/hidden-frame.html?mode=visible`, 2);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: true })).toMatchObject({ ok: true });
  await expectFilled(page, `${BANK}/login.html`);
});
