/**
 * The opt-in save capture runs on every page once it is on (ADR-0038). These pages try to get a
 * login offered from a document the user cannot see or that is not the site's own origin, and
 * to stop the script from installing itself.
 */
import { BANK, expect, open, test, type Fake } from "../fixtures.ts";

const checks = async (fake: Fake): Promise<number> => (await fake.log()).filter((entry) => entry.op === "checkSave").length;

async function settledChecks(fake: Fake): Promise<number> {
  await new Promise((resolve) => setTimeout(resolve, 1200));
  return checks(fake);
}

for (const mode of ["opacity", "filter"] as const) {
  test(`a click into an invisible same-origin frame (${mode}) is not captured`, async ({ page, fake }) => {
    await fake.reset({ items: [] });
    await fake.setCapture(true);
    await open(page, `${BANK}/capture-hidden-frame.html?mode=${mode}`, 2);
    // The user clicks where "Continue" is drawn; the click lands on the invisible frame's submit button.
    await page.frameLocator("#frame").getByRole("button", { name: "Sign in" }).click();
    expect(await settledChecks(fake)).toBe(0);
    expect((await fake.popupState(await fake.tabIdOf(BANK))).pendingSave).toBeNull();
  });
}

test("control: the same frame, visible, is captured", async ({ page, fake }) => {
  await fake.reset({ items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/capture-hidden-frame.html?mode=visible`, 2);
  await page.frameLocator("#frame").getByRole("button", { name: "Sign in" }).click();
  await expect.poll(() => checks(fake)).toBe(1);
});

test("a login submitted in a document with an opaque origin (CSP sandbox) is not offered", async ({ page, fake }) => {
  await fake.reset({ items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/csp-sandbox-capture.html`);
  expect(await page.evaluate(() => self.origin)).toBe("null");
  await page.locator("#user").fill("ada@bank.test");
  await page.locator("#pass").fill("typed-in-the-sandbox");
  await page.getByRole("button", { name: "Sign in" }).click();
  expect(await settledChecks(fake)).toBe(0);
  expect((await fake.popupState(await fake.tabIdOf(BANK))).pendingSave).toBeNull();
});

test("a login submitted in a sandboxed frame of the site is not offered", async ({ page, fake }) => {
  await fake.reset({ items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/capture-sandboxed-frame.html`, 2);
  const frame = page.frameLocator("#frame");
  await frame.locator("#user").fill("ada@bank.test");
  await frame.locator("#pass").fill("typed-in-the-sandboxed-frame");
  await frame.getByRole("button", { name: "Sign in" }).click();
  expect(await settledChecks(fake)).toBe(0);
  expect((await fake.popupState(await fake.tabIdOf(BANK))).pendingSave).toBeNull();
});

test("clobbering the capture's marker does not switch it off", async ({ page, fake }) => {
  await fake.reset({ items: [] });
  await fake.setCapture(true);
  await open(page, `${BANK}/capture-clobber.html`);
  await page.locator("#user").fill("ada@bank.test");
  await page.locator("#pass").fill("typed-on-a-clobbered-page");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect.poll(() => checks(fake)).toBe(1);
  expect((await fake.popupState(await fake.tabIdOf(BANK))).pendingSave).toMatchObject({ username: "ada@bank.test" });
});
