/**
 * Timing attacks on the change fill (M7b): a stale probe (the popup's state was taken, then the
 * tab went elsewhere before the click), a navigation to another origin, a downgrade to http and a
 * same-origin reload between the probe and the delivery, and two concurrent changes on two tabs of
 * the same origin. Defense: the origin comes from the tab at click time, delivery is pinned to the
 * probed frame and document, and each delivery carries the values of the one change it was asked for.
 */
import { BANK, BANK_HTTP, EVIL, PASSWORD, bankItem, expect, open, test, type Fake } from "../fixtures.ts";
import {
  NEW_VALUE,
  attempt,
  changeOutcome,
  expectNoSecretInPage,
  fieldsHolding,
  rotationConfig,
  serveRotationPages,
} from "./rotation-site.ts";

const CHANGE = `${BANK}/rotation-pages/change.html`;

/** Starts a change fill and waits until it stops after its probe; `pending` settles after `release()`. */
async function pausedChange(fake: Fake, tabId: number): Promise<{ pending: Promise<string> }> {
  await fake.pauseAfterProbe();
  const pending = changeOutcome(fake, tabId);
  await expect.poll(() => fake.isPaused()).toBe(true);
  return { pending };
}

test.beforeEach(async ({ page, fake }) => {
  await fake.reset(rotationConfig(bankItem({ allowHttp: true })));
  await serveRotationPages(page);
});

test("a stale probe: the tab went to another origin after the popup opened, then the click", async ({ page, fake }) => {
  await open(page, CHANGE);
  const tabId = await fake.tabIdOf(BANK);
  expect((await fake.popupState(tabId)).rotations).toHaveLength(1);
  await page.goto(`${EVIL}/rotation-pages/change.html`);
  expect(await changeOutcome(fake, tabId)).toMatch(/originMismatch/);
  await expectNoSecretInPage(page);
});

for (const [what, target] of [
  ["another origin", `${EVIL}/rotation-pages/change.html`],
  ["the same page over http", `${BANK_HTTP}/rotation-pages/change.html`],
  ["the same change page again (a new document)", `${CHANGE}?reloaded`],
] as const) {
  test(`navigating to ${what} between the probe and the delivery: nothing arrives`, async ({ page, fake }) => {
    await open(page, CHANGE);
    const tabId = await fake.tabIdOf(BANK);
    const { pending } = await pausedChange(fake, tabId);
    await page.goto(target);
    await fake.release();
    expect(await pending).not.toBe("filled");
    await expectNoSecretInPage(page);
  });
}

test("two tabs of the same origin, two concurrent changes: each tab gets its own change and nothing else", async ({ page, context, fake }) => {
  const OTHER = "Other-Gen-Pw-0042!aBc";
  const second = { attemptId: "a0000000000000002.e2e.0", value: OTHER };
  await fake.reset(rotationConfig(bankItem(), [attempt(), attempt(second)]));
  await open(page, CHANGE);
  const tabA = await fake.tabIdOf(CHANGE);
  const pageB = await context.newPage();
  try {
    await serveRotationPages(pageB);
    await open(pageB, `${CHANGE}?b`);
    const tabB = await fake.tabIdOf(`${CHANGE}?b`);
    expect(tabA).not.toBe(tabB);
    expect((await fake.popupState(tabB)).rotations).toHaveLength(2);

    // Tab A's fill waits after its probe while tab B's fill runs through.
    const { pending: pendingA } = await pausedChange(fake, tabA);
    expect(await changeOutcome(fake, tabB, { attemptId: second.attemptId })).toBe("filled");
    await fake.release();
    expect(await pendingA).toBe("filled");

    expect(await fieldsHolding(page, NEW_VALUE)).toEqual(["new", "confirm"]);
    expect(await fieldsHolding(page, OTHER)).toEqual([]);
    expect(await fieldsHolding(pageB, OTHER)).toEqual(["new", "confirm"]);
    expect(await fieldsHolding(pageB, NEW_VALUE)).toEqual([]);
    expect(await fieldsHolding(pageB, PASSWORD)).toEqual(["current"]);
    expect((await fake.log()).filter((entry) => entry.op === "fillChange")).toHaveLength(2);
  } finally {
    await pageB.close();
  }
});
