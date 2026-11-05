import { BANK, EVIL, bankItem, expect, expectNothingFilled, open, test } from "../fixtures.ts";

test("a context-menu gesture naming a missing frame or another origin's frame fills nothing", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/cross-origin-iframe.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "contextMenu", tabId, frameId: 424242 })).toEqual({ ok: false, error: "noFields" });
  const evilFrame = await fake.frameIdOf(tabId, EVIL);
  expect(await fake.gesture({ kind: "contextMenu", tabId, frameId: evilFrame, menuItemId: "generate" })).toEqual({
    ok: false,
    error: "crossOriginFrame",
  });
  // Neither a credential nor a generated password was asked for.
  expect((await fake.log()).filter((entry) => entry.op === "fill" || entry.op === "generate")).toEqual([]);
  await expectNothingFilled(page);
});

test("the context menu on a same-origin frame still needs the popup confirmation", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/same-origin-iframe.html`, 2);
  const tabId = await fake.tabIdOf(BANK);
  const frameId = await fake.frameIdOf(tabId, `${BANK}/login.html`);

  for (const menuItemId of ["fill", "generate"]) {
    expect(await fake.gesture({ kind: "contextMenu", tabId, frameId, menuItemId })).toEqual({
      ok: false,
      error: "frameNeedsConfirmation",
    });
  }
  expect((await fake.log()).filter((entry) => entry.op === "fill" || entry.op === "generate")).toEqual([]);
  await expectNothingFilled(page);
});
