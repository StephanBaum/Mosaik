import { BANK, EVIL, bankItem, expect, expectFilled, inspect, open, test } from "../fixtures.ts";

test("the fill goes to the top frame and never to a cross-origin frame next to it", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/cross-origin-iframe.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true });
  await expectFilled(page, BANK);
  const evil = (await inspect(page)).find((f) => f.url.startsWith(EVIL));
  expect(evil?.values).toEqual({ user: "", pass: "", code: "" });
  expect(evil?.leaks).toEqual([]);
  expect(await fake.fills()).toEqual([{ op: "fill", origin: BANK, topOrigin: BANK, frame: "top" }]);

  // The context menu on the other origin's field is refused too, without asking the desktop.
  const frameId = await fake.frameIdOf(tabId, EVIL);
  const outcome = await fake.gesture({ kind: "contextMenu", tabId, frameId });
  expect(outcome).toEqual({ ok: false, error: "crossOriginFrame" });
  expect(await fake.fills()).toHaveLength(1);
});

test("a frame of another origin that is the only one with fields is not filled", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/cross-origin-only.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "crossOriginFrame" });
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: true })).toEqual({
    ok: false,
    error: "crossOriginFrame",
  });
  expect(await fake.fills()).toEqual([]);
  const evil = (await inspect(page)).find((f) => f.url.startsWith(EVIL));
  expect(evil?.values).toEqual({ user: "", pass: "", code: "" });
  expect(evil?.leaks).toEqual([]);
});
