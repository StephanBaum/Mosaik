/**
 * Frames and the change fill (M7b): the login's real change form framed by another origin, an
 * A-B-A nesting, and a page whose sign-in form (top) and change form (same-origin frame) are in
 * different frames. Defense: `decideFrame` refuses a cross-origin or hostile-ancestry frame before
 * the desktop is asked, and a frame is filled only with a confirmation that names it.
 */
import { BANK, EVIL, PASSWORD, expect, expectNothingFilled, open, test } from "../fixtures.ts";
import { NEW_VALUE, asked, changeOutcome, fieldsHolding, rotationConfig, serveRotationPages } from "./rotation-site.ts";

test.beforeEach(async ({ page, fake }) => {
  await fake.reset(rotationConfig());
  await serveRotationPages(page);
});

test("the real change page framed by another origin: no card there, and no secret requested", async ({ page, fake }) => {
  await open(page, `${EVIL}/rotation-pages/aba-b.html?inner=${encodeURIComponent(BANK)}`, 2);
  const tabId = await fake.tabIdOf(EVIL);
  expect((await fake.popupState(tabId)).rotations).toEqual([]);
  const frameId = await fake.frameIdOf(tabId, `${BANK}/rotation-pages/change.html`);
  // Even a confirmation naming the framed bank document does not help.
  const { frameTarget } = await fake.popupState(tabId);
  expect(await changeOutcome(fake, tabId, { confirmFrame: frameTarget ?? { frameId, documentId: null } })).toMatch(
    /crossOriginFrame|originMismatch/,
  );
  expect(await asked(fake, "fillChange")).toBe(0);
  await expectNothingFilled(page);
});

test("A-B-A: the login's own origin inside another origin's frame is refused before the desktop is asked", async ({ page, fake }) => {
  await open(page, `${BANK}/rotation-pages/aba.html`, 3);
  const tabId = await fake.tabIdOf(BANK);
  // The top page is the login's own address, so the card is shown; the fill must still refuse.
  expect((await fake.popupState(tabId)).rotations).toHaveLength(1);
  const { frameTarget } = await fake.popupState(tabId);
  expect(await changeOutcome(fake, tabId)).toMatch(/crossOriginFrame/);
  expect(await changeOutcome(fake, tabId, { confirmFrame: frameTarget })).toMatch(/crossOriginFrame/);
  expect(await asked(fake, "fillChange")).toBe(0);
  await expectNothingFilled(page);
});

test("a sign-in form on top and the change form in a frame: the frame the popup confirms decides, nothing else", async ({ page, fake }) => {
  await open(page, `${BANK}/rotation-pages/frame-and-login.html`, 2);
  const tabId = await fake.tabIdOf(BANK);
  // The change form is in the frame, so the fill needs a confirmation of that frame; the popup's
  // own frame target is the sign-in form on top, which is not a confirmation of the frame.
  const { frameTarget } = await fake.popupState(tabId);
  expect(await changeOutcome(fake, tabId, { confirmFrame: frameTarget })).toMatch(/frameNeedsConfirmation/);
  expect(await asked(fake, "fillChange")).toBe(0);
  // A confirmation that names the change frame and its document fills only that frame.
  const frameId = await fake.frameIdOf(tabId, `${BANK}/rotation-pages/change.html`);
  expect(await changeOutcome(fake, tabId, { confirmFrame: { frameId, documentId: null } })).toMatch(/frameNeedsConfirmation/);
  await expectNothingFilled(page);
  expect(await fieldsHolding(page, NEW_VALUE)).toEqual([]);
  expect(await fieldsHolding(page, PASSWORD)).toEqual([]);
});
