import { BANK, bankItem, expect, expectNothingFilled, open, test } from "../fixtures.ts";

test("a login form in a sandboxed srcdoc frame (opaque origin) is not filled", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/sandboxed-srcdoc.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  // Either the browser keeps the script out of the opaque frame (nothing to fill) or the
  // frame reports its opaque origin (refused). Both end without a fill.
  for (const gesture of [
    { kind: "shortcut", tabId },
    { kind: "popupFill", tabId, itemId: "i1", confirmFrame: true },
  ] as const) {
    const outcome = await fake.gesture(gesture);
    expect(outcome).toMatchObject({ ok: false });
    expect(["frameOpaque", "noFields"]).toContain(outcome && !outcome.ok ? outcome.error : "");
  }
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
});
