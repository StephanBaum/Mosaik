import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../../App.tsx";
import { createMockIpc, type MockOptions } from "../../ipc/mock.ts";

async function openHardwareVault(options: MockOptions = {}) {
  const user = userEvent.setup();
  const mock = createMockIpc({ start: "unlocked", quick: ["hardwareKey"], ...options });
  render(<App ipc={mock} languages={["en"]} />);
  await screen.findByText("GitHub");
  // The closed vault is listed by its placeholder name and shows no item names.
  await user.click(await screen.findByRole("button", { name: "Codes (security key)" }));
  return { user };
}

describe("hardware-protected vault", () => {
  it("opens with the PIN, shows the codes and closes again on the closed event", async () => {
    const { user } = await openHardwareVault({ hwOpenSeconds: 1 });
    expect(await screen.findByRole("heading", { name: "This vault needs your security key" })).toBeTruthy();
    expect(screen.queryByText("Bank")).toBeNull();

    await user.type(screen.getByLabelText("Security key PIN"), "1234");
    await user.click(screen.getByRole("button", { name: "Use security key" }));
    expect(await screen.findByText("Bank")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Lock this vault" })).toBeTruthy();
    expect(screen.getByText(/^Locks in \d+:\d\d$/)).toBeTruthy();

    // The mock closes the vault after one second and emits hardwareVaultClosed.
    expect(await screen.findByRole("heading", { name: "This vault needs your security key" })).toBeTruthy();
    expect(screen.queryByText("Bank")).toBeNull();
  });

  it("locks the vault on request", async () => {
    const { user } = await openHardwareVault();
    await user.type(await screen.findByLabelText("Security key PIN"), "1234");
    await user.click(screen.getByRole("button", { name: "Use security key" }));
    await user.click(await screen.findByRole("button", { name: "Lock this vault" }));
    expect(await screen.findByText("Locked. Your other vaults stay unlocked.")).toBeTruthy();
  });

  it("shows a wrong PIN as a sentence and keeps the panel", async () => {
    const { user } = await openHardwareVault();
    await user.type(await screen.findByLabelText("Security key PIN"), "0000");
    await user.click(screen.getByRole("button", { name: "Use security key" }));
    expect((await screen.findAllByRole("alert")).some((a) => /Wrong PIN/.test(a.textContent))).toBe(true);
    expect(screen.getByLabelText<HTMLInputElement>("Security key PIN").value).toBe("");
  });

  it("offers a backup key only while one key is registered", async () => {
    const { user } = await openHardwareVault();
    await user.type(await screen.findByLabelText("Security key PIN"), "1234");
    await user.click(screen.getByRole("button", { name: "Use security key" }));
    const card = await screen.findByRole("region", { name: "Register a backup key" });

    await user.click(within(card).getByRole("button", { name: "Add security key" }));
    const dialog = await screen.findByRole("dialog", { name: "Add security key" });
    await user.type(within(dialog).getByLabelText("Security key PIN"), "1234");
    await user.click(within(dialog).getByRole("button", { name: "Add security key" }));
    await user.click(await screen.findByRole("button", { name: "Keys" }));
    await screen.findByRole("dialog", { name: "Keys for this vault" });
    expect(screen.queryByRole("region", { name: "Register a backup key" })).toBeNull();
  });
});
