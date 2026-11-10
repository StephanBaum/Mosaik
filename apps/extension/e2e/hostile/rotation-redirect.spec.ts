/**
 * "Open change page" and where the site sends the tab (M7b). The change page redirects to a
 * sibling subdomain (related), to another site, to look-alike and IDN hosts, to http, to another
 * port of the same host, and to a sibling tenant of a private public suffix (github.io). Defense:
 * no exact match means no card and a refused fill; the add-origin offer only for the same
 * registrable domain over https in a tab the extension sent, and the desktop decides; the tab's
 * record is dropped when the tab leaves the site.
 */
import type { Page } from "@playwright/test";
import { BANK, BANK_HTTP, EVIL, bankItem, expect, expectNothingFilled, open, test, type Fake } from "../fixtures.ts";
import { HTTPS_PORT } from "../../test/hostile-site/server.mjs";
import { IDS, asked, changeOutcome, rotationConfig, serveRotationPages } from "./rotation-site.ts";

const PORT = String(HTTPS_PORT);
const ACCOUNTS = `https://accounts.bank.test:${PORT}`;

/** The fake desktop's change URL for a login is `<origin>/change-password.html`. */
async function sendToChangePage(page: Page, fake: Fake, from: string, to: string): Promise<number> {
  await serveRotationPages(page, { [`${from}/change-password.html`]: to });
  await open(page, `${from}/rotation-pages/change.html`);
  const tabId = await fake.tabIdOf(from);
  expect((await fake.popupState(tabId)).rotations).toHaveLength(1);
  await fake.popupRequest(tabId, { op: "rotationOpenPage", ...IDS });
  await page.waitForURL(to);
  await page.waitForLoadState("load");
  return tabId;
}

const addOrigin = (fake: Fake, tabId: number): Promise<string> =>
  fake.popupRequest(tabId, { op: "rotationAddOrigin" }).then(
    (result) => JSON.stringify(result),
    (error: unknown) => (error instanceof Error ? error.message : String(error)),
  );

test("a redirect to a sibling subdomain: no card, a refused fill, then the offer and the desktop decides", async ({ page, fake }) => {
  await fake.reset(rotationConfig());
  const tabId = await sendToChangePage(page, fake, BANK, `${ACCOUNTS}/rotation-pages/change.html`);

  const state = await fake.popupState(tabId);
  expect(state.rotations).toEqual([]);
  expect(state.rotationTabOffer).toMatchObject({ host: "accounts.bank.test", title: "Bank" });
  expect(await changeOutcome(fake, tabId)).toMatch(/originMismatch/);
  await expectNothingFilled(page);

  expect(await addOrigin(fake, tabId)).toContain("added");
  expect((await fake.log()).filter((entry) => entry.op === "rotationAddOrigin")).toEqual([
    { op: "rotationAddOrigin", origin: ACCOUNTS, topOrigin: ACCOUNTS, frame: null },
  ]);
  // After the desktop added it, the address is the login's own: the card and the fill follow.
  expect((await fake.popupState(tabId)).rotations).toHaveLength(1);
  expect(await changeOutcome(fake, tabId)).toBe("filled");
});

const unrelated = [
  ["another site", `${EVIL}/rotation-pages/change.html`],
  ["a look-alike that ends in another site", `https://bank.test.evil.test:${PORT}/rotation-pages/change.html`],
  ["a typo look-alike", `https://bamk.test:${PORT}/rotation-pages/change.html`],
  ["an IDN look-alike (Cyrillic a)", `https://xn--bnk-6cd.test:${PORT}/rotation-pages/change.html`],
] as const;

for (const [what, target] of unrelated) {
  test(`a redirect to ${what}: no card, no offer, a forged add refused, nothing filled`, async ({ page, fake }) => {
    await fake.reset(rotationConfig());
    const tabId = await sendToChangePage(page, fake, BANK, target);
    const state = await fake.popupState(tabId);
    expect(state.rotations).toEqual([]);
    expect(state.rotationTabOffer).toBeNull();
    expect(await changeOutcome(fake, tabId)).toMatch(/originMismatch/);
    expect(await addOrigin(fake, tabId)).toMatch(/originNotRelated/);
    expect(await asked(fake, "rotationAddOrigin")).toBe(0);
    await expectNothingFilled(page);
  });
}

test("a redirect to http: refused even with the login's HTTP override", async ({ page, fake }) => {
  await fake.reset(rotationConfig(bankItem({ allowHttp: true })));
  const tabId = await sendToChangePage(page, fake, BANK, `${BANK_HTTP}/rotation-pages/change.html`);
  const state = await fake.popupState(tabId);
  expect(state.rotations).toEqual([]);
  expect(state.rotationTabOffer).toBeNull();
  expect(await changeOutcome(fake, tabId)).toMatch(/insecureOrigin/);
  expect(await addOrigin(fake, tabId)).toMatch(/insecureOrigin/);
  expect(await asked(fake, "rotationAddOrigin")).toBe(0);
  await expectNothingFilled(page);
});

test("a page that sends itself to a sibling subdomain (not the extension) gets no offer", async ({ page, fake }) => {
  await fake.reset(rotationConfig());
  await serveRotationPages(page);
  await open(page, `${BANK}/rotation-pages/change.html`);
  await page.evaluate((url) => { location.href = url; }, `${ACCOUNTS}/rotation-pages/change.html`);
  await page.waitForURL(`${ACCOUNTS}/**`);
  const tabId = await fake.tabIdOf(ACCOUNTS);
  expect((await fake.popupState(tabId)).rotationTabOffer).toBeNull();
  expect(await addOrigin(fake, tabId)).toMatch(/originNotRelated/);
});

// Was a break (low), fixed in the M7b review round: the record is dropped only when the popup happens to be opened while the tab is
// on another site (`RotationService.#offer`); a tab that leaves for another site and comes back
// to the login's site later still gets the offer (spec §4.2: "cleared ... on leaving the
// registrable domain").
test("a tab that left the site after Open change page gets no offer when it comes back", async ({ page, fake }) => {
  await fake.reset(rotationConfig());
  const tabId = await sendToChangePage(page, fake, BANK, `${BANK}/rotation-pages/change.html?landed`);
  await page.goto(`${EVIL}/rotation-pages/change.html`);
  await page.goto(`${ACCOUNTS}/rotation-pages/change.html`);
  expect((await fake.popupState(tabId)).rotationTabOffer).toBeNull();
});

// Was a break (low), fixed in the M7b review round: the offer names the host without the port. A redirect to another port of the
// login's own host reads "This page is on bank.test, which isn't saved for this login. Add it?"
// while the login is saved for bank.test, so the user cannot see what is being added.
test("a redirect to another port of the same host is never offered as the login's own host name", async ({ page, fake }) => {
  await fake.reset(rotationConfig());
  const tabId = await sendToChangePage(page, fake, BANK, "https://bank.test:4999/rotation-pages/change.html");
  const offer = (await fake.popupState(tabId)).rotationTabOffer;
  expect(offer === null || offer.host !== "bank.test").toBe(true);
});

// Was a break (low), fixed in the M7b review round: `siteOf` (src/shared/origin.ts) guesses the registrable domain from the last two
// labels, so two tenants of a private public suffix (alice.github.io, mallory.github.io) count as
// one site: the popup offers to add the other tenant. The desktop refuses (`originNotRelated`, it
// uses the full list), but the trusted UI made the offer and the comment says it "errs towards
// not related", which it does not here.
test("a redirect to a sibling tenant of a private public suffix gets no offer", async ({ page, fake }) => {
  const alice = "https://alice.github.io";
  await fake.reset(rotationConfig(bankItem({ origin: alice })));
  const tabId = await sendToChangePage(page, fake, alice, "https://mallory.github.io/rotation-pages/change.html");
  expect((await fake.popupState(tabId)).rotationTabOffer).toBeNull();
});
