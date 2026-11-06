import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`Browser extension prompts, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("the pairing dialog shows the code, focuses Deny and lets Allow wait a second", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserPair=1");
      const deny = page.getByRole("button", { name: "Deny" });
      await expect(deny).toBeVisible({ timeout: 5000 });
      await expect(deny).toBeFocused();
      await expect(page.getByText("4F7-K2Q")).toBeVisible();
      await expect(page.getByRole("button", { name: "Allow", exact: true })).toBeDisabled();
      await expectAccessible(page);
      await expect(page.getByRole("button", { name: "Allow", exact: true })).toBeEnabled({ timeout: 3000 });
      await page.getByRole("button", { name: "Allow", exact: true }).click();
      await expect(deny).toBeHidden();
      guard.expectClean();
    });

    test("the save dialog for a new login on an unencrypted page", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserSave=new");
      const cancel = page.getByRole("button", { name: "Cancel" });
      await expect(cancel).toBeVisible({ timeout: 5000 });
      await expect(cancel).toBeFocused();
      await expect(page.getByText("••••••••")).toBeVisible();
      await expect(page.getByRole("switch", { name: "Allow filling on this unencrypted page" })).toBeVisible();
      await expectAccessible(page);
      await expect(page.getByRole("button", { name: "Save", exact: true })).toBeEnabled({ timeout: 3000 });
      await page.getByRole("button", { name: "Save", exact: true }).click();
      await expect(cancel).toBeHidden();
      guard.expectClean();
    });

    test("the save dialog for an update", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserSave=update");
      await expect(page.getByRole("combobox", { name: "Save as" })).toBeVisible({ timeout: 5000 });
      await expectAccessible(page);
      await page.getByRole("button", { name: "Cancel" }).click();
      await expect(page.getByRole("combobox", { name: "Save as" })).toBeHidden();
      guard.expectClean();
    });

    test("the passkey sign-in dialog lists the accounts and lets Sign in wait a second", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserPasskey=get");
      const cancel = page.getByRole("button", { name: "Cancel" });
      await expect(cancel).toBeVisible({ timeout: 5000 });
      await expect(cancel).toBeFocused();
      await expect(page.getByRole("heading", { name: "Sign in to example.com with a passkey" })).toBeVisible();
      await expect(page.getByText("Secure connection (https)")).toBeVisible();
      await expect(page.getByRole("radio")).toHaveCount(2);
      await expect(page.getByRole("radio").first()).toBeChecked();
      await expect(page.getByRole("button", { name: "Use another device or security key" })).toBeVisible();
      await expect(page.getByRole("button", { name: "Sign in with passkey" })).toBeDisabled();
      await expectAccessible(page);
      await expect(page.getByRole("button", { name: "Sign in with passkey" })).toBeEnabled({ timeout: 3000 });
      await page.getByRole("button", { name: "Sign in with passkey" }).click();
      await expect(cancel).toBeHidden();
      guard.expectClean();
    });

    test("the passkey dialog asks for the master password when the site wants verification", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserPasskey=uv");
      const password = page.getByLabel("Master password");
      await expect(password).toBeVisible({ timeout: 5000 });
      await expectAccessible(page);
      await password.fill("mosaic");
      await expect(page.getByRole("button", { name: "Sign in with passkey" })).toBeEnabled({ timeout: 3000 });
      await page.getByRole("button", { name: "Sign in with passkey" }).click();
      await expect(password).toBeHidden();
      guard.expectClean();
    });

    test("the passkey create dialog saves into a vault, and an exclusion only informs", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserPasskey=create");
      await expect(page.getByRole("heading", { name: "Save a new passkey" })).toBeVisible({ timeout: 5000 });
      await expect(page.getByText("Create a passkey for example.com in Mosaic?")).toBeVisible();
      await expect(page.getByRole("combobox", { name: "Vault" })).toBeVisible();
      await expectAccessible(page);
      await expect(page.getByRole("button", { name: "Save passkey" })).toBeEnabled({ timeout: 3000 });
      await page.getByRole("button", { name: "Save passkey" }).click();
      await expect(page.getByRole("heading", { name: "Save a new passkey" })).toBeHidden();

      await page.goto("/?mock=unlocked&browserPasskey=excluded");
      await expect(page.getByText("You already have a passkey for example.com in Mosaic.")).toBeVisible({
        timeout: 5000,
      });
      await expect(page.getByRole("button", { name: "Save passkey" })).toHaveCount(0);
      await expectAccessible(page);
      // The acknowledge waits out the Allow guard like an Allow.
      await expect(page.getByRole("button", { name: "OK" })).toHaveAttribute("aria-disabled", "false", {
        timeout: 3000,
      });
      await page.getByRole("button", { name: "OK" }).click();
      await expect(page.getByRole("heading", { name: "Passkey already saved" })).toBeHidden();
      guard.expectClean();
    });

    test("a passkey item shows the site, the account and a note about its key", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserPasskey=get");
      await expect(page.getByRole("button", { name: "Cancel" })).toBeVisible({ timeout: 5000 });
      await page.getByRole("button", { name: "Cancel" }).click();
      await page.getByRole("button", { name: /example\.com.*alex@example\.com/ }).first().click();
      const article = page.getByRole("article", { name: "example.com" });
      await expect(article.getByText("Credential ID")).toBeVisible();
      await expect(article.getByText("ES256")).toBeVisible();
      await expect(article.getByText(/private key never leaves Mosaic/)).toBeVisible();
      await expectAccessible(page);
      guard.expectClean();
    });

    test("Settings lists the paired browsers", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked");
      await page.getByRole("button", { name: "Settings" }).click();
      await expect(page.getByRole("heading", { name: "Browsers" })).toBeVisible();
      await expect(page.getByRole("button", { name: "Disconnect Chrome" })).toBeVisible();
      await expectAccessible(page);
      await page.getByRole("button", { name: "Disconnect Chrome" }).click();
      await expect(page.getByText("Disconnect “Chrome”?")).toBeVisible();
      await expectAccessible(page);
      guard.expectClean();
    });
  });
}
