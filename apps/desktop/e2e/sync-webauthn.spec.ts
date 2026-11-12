import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

// The mock's browser step ends on its second status poll (two seconds apart): the user "touches
// the key". The real step runs in the system browser on the server's page.
for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`sync security keys, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("sign in again with a security key", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&sync=signedOut&syncWebauthn=only");
      await expect(page.getByText("GitHub")).toBeVisible();
      await page.getByRole("button", { name: "Settings" }).click();
      await page.getByRole("textbox", { name: "Master password" }).fill("mosaic");
      await page.getByRole("button", { name: "Sign in again" }).click();

      // A key-only account: the key, no code field; focus is on the step's heading.
      const choice = page.getByRole("heading", { name: "Use your security key" });
      await expect(choice).toBeFocused();
      await expect(page.getByLabel("6-digit code")).toHaveCount(0);
      await expectAccessible(page);
      await page.getByRole("button", { name: "Use security key (opens your browser)" }).click();

      // The code is big, readable by a screen reader, and the status is a live region.
      const step = page.getByRole("heading", { name: "Sign in with your security key" });
      await expect(step).toBeFocused();
      await expect(page.locator(".handoff__code")).toContainText("482913");
      await expect(page.getByText("Code to type in your browser: 4 8 2 9 1 3")).toBeAttached();
      await expect(page.getByRole("status").filter({ hasText: "Waiting for your security key" })).toBeVisible();
      await expect(page.getByRole("button", { name: "Open the browser page again" })).toBeVisible();
      await expectAccessible(page);

      // The page finishes the sign-in on its own.
      await expect(page.getByRole("button", { name: "Sync now" })).toBeVisible({ timeout: 15_000 });
      guard.expectClean();
    });

    test("add and remove a security key", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&sync=hosted&syncWebauthn=both");
      await expect(page.getByText("GitHub")).toBeVisible();
      await page.getByRole("button", { name: "Settings" }).click();
      await expect(page.getByText("YubiKey 5C")).toBeVisible();
      await expectAccessible(page);

      await page.getByRole("button", { name: "Add a security key…" }).click();
      const dialog = page.getByRole("dialog");
      await dialog.getByLabel("Name of the key").fill("Desk key");
      await dialog.getByLabel("Master password").fill("mosaic");
      await dialog.getByLabel("6-digit code").fill("123456");
      await expectAccessible(page);
      await dialog.getByRole("button", { name: "Continue" }).click();
      await expect(dialog.getByRole("heading", { name: "Add your security key" })).toBeFocused();
      await expect(dialog.locator(".handoff__code")).toContainText("482913");
      await expectAccessible(page);
      await expect(page.getByText("Security key added.")).toBeVisible({ timeout: 15_000 });
      await expect(page.getByText("Desk key")).toBeVisible();

      // Removing needs a second step first: the master password, then a key in the browser.
      await page.getByRole("button", { name: "Remove Desk key" }).click();
      await page.getByRole("dialog").getByRole("button", { name: "Remove key" }).click();
      await page.getByRole("dialog").getByLabel("Master password").fill("mosaic");
      await page.getByRole("dialog").getByRole("button", { name: "Continue" }).click();
      await expect(page.getByText("Security key removed.")).toBeVisible({ timeout: 30_000 });
      await expect(page.getByText("Desk key")).toHaveCount(0);
      guard.expectClean();
    });
  });
}
