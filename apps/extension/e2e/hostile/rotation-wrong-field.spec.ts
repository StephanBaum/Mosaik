/**
 * Hostile change forms (M7b): a page that rearranges its fields between the probe and the fill (in
 * the top page and in a same-origin frame, including a frame that navigates in between), and decoy
 * fields that would pull the current or the new password where the user cannot see it or where
 * the page shows it in clear text. Defense: the content script classifies again at fill time and
 * refuses any difference; a form it cannot read gets no secret requested at all.
 */
import type { Page } from "@playwright/test";
import { BANK, EVIL, PASSWORD, bankItem, expect, expectNothingFilled, inspect, open, test, type Fake } from "../fixtures.ts";
import {
  IDS,
  NEW_VALUE,
  asked,
  changeOutcome,
  expectNoSecretInPage,
  fieldsHolding,
  rotationConfig,
  serveRotationPages,
} from "./rotation-site.ts";

const PAGES = `${BANK}/rotation-pages`;

/** Starts a change fill, waits for the probe, runs `between` on the page, then lets the fill go on. */
async function afterProbe(
  fake: Fake,
  tabId: number,
  between: () => Promise<unknown>,
  confirmFrame: { frameId: number; documentId: string | null } | null = null,
): Promise<string> {
  await fake.pauseAfterProbe();
  const pending = changeOutcome(fake, tabId, { confirmFrame });
  await expect.poll(() => fake.isPaused()).toBe(true);
  await between();
  await fake.release();
  return pending;
}

const call = (page: Page, fn: "swap" | "reorder" | "renew" | "hideNew") =>
  page.evaluate((name) => (window as unknown as Record<string, () => void>)[name]?.(), fn);

test.beforeEach(async ({ page, fake }) => {
  await fake.reset(rotationConfig());
  await serveRotationPages(page);
});

for (const mutation of ["swap", "renew", "hideNew"] as const) {
  test(`a top page that runs ${mutation}() after the probe receives nothing`, async ({ page, fake }) => {
    await open(page, `${PAGES}/change.html`);
    const tabId = await fake.tabIdOf(BANK);
    expect(await afterProbe(fake, tabId, () => call(page, mutation))).toMatch(/changeFormAmbiguous/);
    await expectNothingFilled(page);
  });
}

test("moving a field whose role comes from its token keeps each value in its own field", async ({ page, fake }) => {
  await open(page, `${PAGES}/change.html`);
  const tabId = await fake.tabIdOf(BANK);
  expect(await afterProbe(fake, tabId, () => call(page, "reorder"))).toBe("filled");
  expect(await fieldsHolding(page, NEW_VALUE)).toEqual(["new", "confirm"]);
  expect(await fieldsHolding(page, PASSWORD)).toEqual(["current"]);
});

test("three untagged fields reordered after the probe (current would land in new) receive nothing", async ({ page, fake }) => {
  await open(page, `${PAGES}/notokens.html`);
  const tabId = await fake.tabIdOf(BANK);
  expect(await afterProbe(fake, tabId, () => call(page, "reorder"))).toMatch(/changeFormAmbiguous/);
  await expectNothingFilled(page);
});

test.describe("a change form in a same-origin frame", () => {
  const changeFrame = (page: Page) => {
    const frame = page.frames().find((f) => f.url().includes("/rotation-pages/change.html"));
    if (frame === undefined) {
      throw new Error("no change frame");
    }
    return frame;
  };

  test("needs the frame confirmation, and with it gets the change pair", async ({ page, fake }) => {
    await open(page, `${PAGES}/frame-host.html`, 2);
    const tabId = await fake.tabIdOf(BANK);
    expect(await changeOutcome(fake, tabId)).toMatch(/frameNeedsConfirmation/);
    expect(await asked(fake, "fillChange")).toBe(0);
    const { frameTarget } = await fake.popupState(tabId);
    expect(frameTarget).not.toBeNull();
    expect(await changeOutcome(fake, tabId, { confirmFrame: frameTarget })).toBe("filled");
    expect(await fieldsHolding(page, NEW_VALUE)).toEqual(["new", "confirm"]);
  });

  test("swapped roles inside the confirmed frame after the probe: nothing", async ({ page, fake }) => {
    await open(page, `${PAGES}/frame-host.html`, 2);
    const tabId = await fake.tabIdOf(BANK);
    const { frameTarget } = await fake.popupState(tabId);
    const outcome = await afterProbe(fake, tabId, () =>
        changeFrame(page).evaluate(() => {
          (window as unknown as { swap: () => void }).swap();
        }), frameTarget);
    expect(outcome).toMatch(/changeFormAmbiguous/);
    await expectNothingFilled(page);
  });

  for (const [what, target] of [
    ["the same change page again", `${PAGES}/change.html?again`],
    ["another origin", `${EVIL}/rotation-pages/change.html`],
  ] as const) {
    test(`the confirmed frame navigating to ${what} after the probe receives nothing`, async ({ page, fake }) => {
      await open(page, `${PAGES}/frame-host.html`, 2);
      const tabId = await fake.tabIdOf(BANK);
      const { frameTarget } = await fake.popupState(tabId);
      const outcome = await afterProbe(
        fake,
        tabId,
        async () => {
          await changeFrame(page).evaluate((url) => { location.href = url; }, target);
          await expect.poll(() => page.frames().some((f) => f.url() === target)).toBe(true);
          await page.frames().find((f) => f.url() === target)?.waitForLoadState("load");
        },
        frameTarget,
      );
      expect(outcome).not.toBe("filled");
      await expectNothingFilled(page);
    });
  }
});

test.describe("decoy fields", () => {
  for (const decoy of ["offscreen", "opacity", "zero", "clip", "textdecoy", "overlay"] as const) {
    test(`a change form with the ${decoy} decoy gets no secret requested`, async ({ page, fake }) => {
      await open(page, `${PAGES}/decoys.html?v=${decoy}`);
      const tabId = await fake.tabIdOf(BANK);
      expect(await changeOutcome(fake, tabId)).toMatch(/changeFormAmbiguous/);
      expect(await asked(fake, "fillChange")).toBe(0);
      await expectNothingFilled(page);
    });
  }

  test("a hidden second form with a new-password field: ambiguous without focus, and never filled", async ({ page, fake }) => {
    await open(page, `${PAGES}/decoys.html?v=otherform`);
    const tabId = await fake.tabIdOf(BANK);
    expect(await changeOutcome(fake, tabId)).toMatch(/changeFormAmbiguous/);
    expect(await asked(fake, "fillChange")).toBe(0);
    // The user works in the visible form: only its fields are filled.
    await page.locator("#current").focus();
    expect(await changeOutcome(fake, tabId)).toBe("filled");
    expect(await fieldsHolding(page, NEW_VALUE)).toEqual(["new", "confirm"]);
    expect((await inspect(page))[0]?.values["decoy"]).toBe("");
  });

  // Was a break (low), fixed in the M7b review round: a `type=text` field that the page declares `autocomplete="new-password"` counts
  // as a show-password toggle even when it is the user name field (`name=username`) and a real
  // password-type new field sits next to it: the new password lands in clear text in the user
  // name field, against "never the user name" (spec §4.2 point 4, wrong-field case d).
  test("the user name field declaring new-password never receives the new password", async ({ page, fake }) => {
    await open(page, `${PAGES}/decoys.html?v=usernew`);
    const tabId = await fake.tabIdOf(BANK);
    const outcome = await changeOutcome(fake, tabId);
    expect(await fieldsHolding(page, NEW_VALUE)).not.toContain("user");
    expect(outcome).toMatch(/changeFormAmbiguous/);
  });
});

test("a request naming another login's change gets nothing into the page", async ({ page, fake }) => {
  await fake.reset(
    rotationConfig(bankItem(), [
      { ...IDS, value: NEW_VALUE, stage: "prepared" },
    ]),
  );
  await fake.configure({
    items: [bankItem(), bankItem({ itemId: "i2", title: "Other", origin: EVIL, password: "other secret" })],
  });
  await open(page, `${PAGES}/change.html`);
  const tabId = await fake.tabIdOf(BANK);
  expect(await changeOutcome(fake, tabId, { itemId: "i2" })).toMatch(/originMismatch|rotationNotFound/);
  expect(await changeOutcome(fake, tabId, { attemptId: "a0000000000000009.e2e.0" })).toMatch(/rotationNotFound/);
  await expectNoSecretInPage(page, [NEW_VALUE, PASSWORD, "other secret"]);
});
