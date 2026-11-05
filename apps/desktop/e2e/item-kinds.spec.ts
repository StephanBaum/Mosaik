import { expect, test, type Page } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

async function save(page: Page) {
  await page.getByRole("button", { name: "Save" }).click();
}

test.describe("item kinds and custom fields", () => {
  test.use({ locale: "en-US" });

  test("a payment card with a hidden custom field, from create to remove", async ({ page }) => {
    const guard = watch(page);
    await page.goto("/?mock=unlocked");
    await expect(page.getByText("GitHub")).toBeVisible();

    // 1. A payment card: the Luhn hint, the expiry, the grouped number.
    await page.keyboard.press("Control+n");
    await page.getByLabel("Type", { exact: true }).selectOption("creditCard");
    await expect(page.getByLabel("User name")).toHaveCount(0);
    await page.getByLabel("Title").fill("Travel card");
    await page.getByLabel("Cardholder").fill("Alex Example");
    await page.getByLabel("Card number").fill("4111 1111 1111 1112");
    await expect(page.getByText("This number doesn't pass the card check. Check it for typos.")).toBeVisible();
    await page.getByLabel("Card number").fill("4111 1111 1111 1111");
    await expect(page.getByText("This number doesn't pass the card check. Check it for typos.")).toBeHidden();
    await expect(page.getByLabel("Card brand")).toHaveValue("Visa");
    await page.getByLabel("Expires").fill("0429");
    await expectAccessible(page);
    await save(page);

    const card = page.getByRole("article", { name: "Travel card" });
    await expect(card).toBeVisible();
    await expect(card.getByText("04/2029")).toBeVisible();
    await expect(card.getByText("Expired")).toHaveCount(0);
    await card.getByRole("button", { name: "Reveal Card number" }).click();
    await expect(card.getByText("4111 1111 1111 1111")).toBeVisible();
    await expectAccessible(page);

    // 2. A hidden custom field.
    await card.getByRole("button", { name: "Edit" }).click();
    await page.getByRole("button", { name: "Add custom field" }).click();
    await page.getByLabel("Field name", { exact: true }).fill("Door code");
    await page.getByLabel("Field type", { exact: true }).selectOption("hidden");
    await page.getByLabel("Value", { exact: true }).fill("1234");
    await expectAccessible(page);
    await save(page);
    await card.getByRole("button", { name: "Reveal Door code" }).click();
    await expect(card.getByText("1234", { exact: true })).toBeVisible();

    // 3. Rename it without touching the value: there is no kind picker while editing.
    await card.getByRole("button", { name: "Edit" }).click();
    await expect(page.getByLabel("Type", { exact: true })).toHaveCount(0);
    await page.getByLabel("Field name", { exact: true }).fill("Gate code");
    await save(page);
    await card.getByRole("button", { name: "Reveal Gate code" }).click();
    await expect(card.getByText("1234", { exact: true })).toBeVisible();

    // 4. Remove it.
    await card.getByRole("button", { name: "Edit" }).click();
    await page.getByRole("button", { name: "Remove Gate code" }).click();
    await save(page);
    await expect(card.getByText("Gate code")).toHaveCount(0);
    await expectAccessible(page);
    guard.expectClean();
  });

  test("an SSH key: paste once, see the public key, never the private one", async ({ page }) => {
    await page.goto("/?mock=unlocked");
    await expect(page.getByText("GitHub")).toBeVisible();
    await page.keyboard.press("Control+n");
    await page.getByLabel("Type", { exact: true }).selectOption("sshKey");
    await page.getByLabel("Title").fill("Build server");
    await page.getByLabel("Private key", { exact: true }).fill("hunter2");
    await save(page);
    await expect(page.getByText(/This isn't an OpenSSH private key/)).toBeVisible();
    await page
      .getByLabel("Private key", { exact: true })
      .fill("-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----");
    await expectAccessible(page);
    await save(page);
    const key = page.getByRole("article", { name: "Build server" });
    await expect(key.getByText("SHA256:", { exact: false })).toBeVisible();
    await expect(key.getByRole("button", { name: "Copy Public key" })).toBeVisible();
    await expect(key.getByText("BEGIN OPENSSH")).toHaveCount(0);
    await expectAccessible(page);
  });
});
