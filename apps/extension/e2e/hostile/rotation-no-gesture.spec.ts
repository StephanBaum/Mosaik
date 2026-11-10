/**
 * No change fill without a click in the popup (M7b, OD-M7b-8): a change page that focuses its
 * fields on load, fires synthetic clicks and the fill shortcut as synthetic keys, and forges
 * fill / result / add-origin messages, also from a cross-origin frame; the real shortcut and
 * context menu on a change page; and a page that frames Mosaic's popup to clickjack the card.
 * Defense: no listener for the page, popup-port-only rotation ops, a popup page that cannot be
 * framed, and a login fill (shortcut, menu) that never carries the new password.
 */
import { BANK, EVIL, PASSWORD, expect, expectNothingFilled, inspect, open, test } from "../fixtures.ts";
import { NEW_VALUE, expectNoSecretInPage, fieldsHolding, rotationConfig, serveRotationPages } from "./rotation-site.ts";

const ROTATION_OPS = ["rotationForOrigin", "rotationBegin", "fillChange", "fillPending", "rotationResult", "rotationAddOrigin"];

test.beforeEach(async ({ page, fake }) => {
  await fake.reset(rotationConfig());
  await serveRotationPages(page);
});

test("focus, synthetic clicks and keys, and forged messages on load start no change fill", async ({ page, fake }) => {
  await open(page, `${BANK}/rotation-pages/no-gesture.html`);
  await page.evaluate(() => {
    (window as unknown as { attack: () => void }).attack();
  });
  await page.waitForTimeout(1500);
  expect(await page.evaluate(() => (window as unknown as { __runtime: string }).__runtime)).toBe("absent");
  await expectNothingFilled(page);
  expect(await fake.log()).toEqual([]);
});

test("a cross-origin frame forging change messages at the login's page starts nothing", async ({ page, fake }) => {
  await open(page, `${BANK}/rotation-pages/change.html`);
  await page.evaluate((src) => {
    const frame = document.createElement("iframe");
    frame.src = src;
    document.body.append(frame);
  }, `${EVIL}/rotation-pages/no-gesture.html`);
  await expect.poll(() => page.frames().length).toBe(2);
  await page.frames()[1]?.waitForLoadState("load");
  await page.waitForTimeout(1500);
  await expectNothingFilled(page);
  expect(await fake.log()).toEqual([]);
});

test("the real shortcut and context menu on a change page never carry the new password", async ({ page, fake }) => {
  await open(page, `${BANK}/rotation-pages/change.html`);
  const tabId = await fake.tabIdOf(BANK);
  await fake.gesture({ kind: "shortcut", tabId });
  await fake.gesture({ kind: "contextMenu", tabId });
  expect(await fieldsHolding(page, NEW_VALUE)).toEqual([]);
  for (const frame of await inspect(page)) {
    expect(frame.leaks.map((leak) => leak.value)).not.toContain(NEW_VALUE);
  }
  // Whatever the login fill did, it was a login fill: the desktop was asked no change op.
  expect((await fake.log()).map((entry) => entry.op).filter((op) => ROTATION_OPS.includes(op) && op !== "rotationForOrigin")).toEqual([]);
  // A login fill on a change form puts the current password at most into the current field.
  expect((await fieldsHolding(page, PASSWORD)).filter((id) => id !== "current")).toEqual([]);
});

test("a page that frames the popup (clickjacking the card) gets no popup", async ({ page, ext, fake }) => {
  await open(page, `${BANK}/rotation-pages/change.html`);
  const tabId = await fake.tabIdOf(BANK);
  await page.goto(`${EVIL}/rotation-pages/popup-frame.html?ext=${ext.id}&tab=${String(tabId)}`);
  await expect.poll(() => page.frames().length).toBe(2);
  await page.waitForTimeout(1000);
  const framed = page.frames()[1];
  // The browser refuses to put the popup page in a web page's frame (not web-accessible,
  // frame-ancestors 'none'): the frame is an error page, never the popup.
  const rendered = await framed
    ?.evaluate(() => document.body.innerText)
    .catch(() => "");
  expect(rendered ?? "").not.toContain("Password change");
  expect(rendered ?? "").not.toContain("Fill change form");
  // The framed page never connected: the desktop was asked nothing on its behalf.
  expect(await fake.log()).toEqual([]);
  await expectNoSecretInPage(page);
});
