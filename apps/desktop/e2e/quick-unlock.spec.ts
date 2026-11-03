import { expect, test, type Page } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

const WINDOWS_UA =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

async function openCodesVault(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Codes (security key)" }).click();
  await expect(page.getByRole("heading", { name: "This vault needs your security key" })).toBeVisible();
}

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`quick unlock, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US", userAgent: WINDOWS_UA });

    test("unlocks with Windows Hello", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=locked&quick=biometric");
      await expect(page.getByRole("button", { name: "Use Windows Hello" })).toBeVisible();
      await expectAccessible(page);

      await page.getByRole("button", { name: "Use Windows Hello" }).click();
      await expect(page.getByText("GitHub")).toBeVisible();
      guard.expectClean();
    });

    test("unlocks with a security key and its PIN", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=locked&quick=hardwareKey");
      await page.getByRole("button", { name: "Use security key" }).click();
      await expect(page.getByLabel("Security key PIN")).toBeVisible();
      await expectAccessible(page);

      await page.getByLabel("Security key PIN").fill("1234");
      await page.getByRole("button", { name: "Continue" }).click();
      await expect(page.getByText("GitHub")).toBeVisible();
      guard.expectClean();
    });

    test("explains an expired quick unlock and keeps the password form", async ({ page }) => {
      await page.goto("/?mock=locked&quick=biometric&quickFail=quickUnlockExpired");
      await page.getByRole("button", { name: "Use Windows Hello" }).click();
      await expect(page.getByText(/Quick unlock expired/)).toBeVisible();
      await expect(page.getByRole("button", { name: "Use Windows Hello" })).toHaveCount(0);
      await expect(page.getByLabel("Master password")).toBeFocused();
      await expectAccessible(page);
    });

    test("settings: enable Windows Hello, change the max age, remove a key", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&quick=hardwareKey");
      await page.getByRole("button", { name: "Settings" }).click();
      await expect(page.getByRole("heading", { name: "Quick unlock" })).toBeVisible();
      await expectAccessible(page);

      await page.getByRole("switch", { name: "Windows Hello" }).click();
      const dialog = page.getByRole("dialog", { name: "Confirm with your master password" });
      await expect(dialog).toBeVisible();
      await expectAccessible(page);
      await dialog.getByLabel("Master password", { exact: true }).fill("mosaic");
      await dialog.getByRole("button", { name: "Confirm" }).click();
      await expect(page.getByText(/Asks for your master password again on/)).toBeVisible();

      await page.getByLabel("Ask for the master password at least every").selectOption("7");
      await expect(page.getByText("Settings saved.").last()).toBeVisible();

      await page.getByRole("button", { name: "Manage" }).click();
      const keys = page.getByRole("dialog", { name: "Security keys" });
      await expect(keys.getByText(/Security key · 01234567 · added/)).toBeVisible();
      await expectAccessible(page);
      await keys.getByRole("button", { name: "Remove security key 01234567" }).click();
      await expect(keys.getByText("None registered")).toBeVisible();
      guard.expectClean();
    });

    test("hardware vault: open with key and PIN, see codes, lock it again", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&quick=hardwareKey");
      await openCodesVault(page);
      await expectAccessible(page);

      await page.getByLabel("Security key PIN").fill("1234");
      await page.getByRole("button", { name: "Use security key" }).click();
      await expect(page.getByText("Bank", { exact: true })).toBeVisible();
      await expect(page.getByRole("region", { name: "Register a backup key" })).toBeVisible();
      await expectAccessible(page);

      await page.getByRole("button", { name: "Lock this vault" }).click();
      await expect(page.getByRole("heading", { name: "This vault needs your security key" })).toBeVisible();
      await expect(page.getByText("Bank", { exact: true })).toHaveCount(0);
      guard.expectClean();
    });
  });
}
