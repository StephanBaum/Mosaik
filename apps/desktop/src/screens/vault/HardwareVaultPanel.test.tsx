import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../../App.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import { createMockIpc, type MockOptions } from "../../ipc/mock.ts";

type User = ReturnType<typeof userEvent.setup>;

async function openHardwareVault(options: MockOptions = {}) {
  const user = userEvent.setup();
  const calls: { command: string; request: unknown }[] = [];
  const mock = createMockIpc({ start: "unlocked", quick: ["hardwareKey"], ...options });
  const ipc: MosaicIpc = {
    subscribe: (listener) => mock.subscribe(listener),
    call: (command, request) => {
      calls.push({ command, request });
      return mock.call(command, request);
    },
  };
  render(<App ipc={ipc} languages={["en"]} />);
  await screen.findByText("GitHub");
  // The closed vault is listed by its placeholder name and shows no item names.
  await user.click(await screen.findByRole("button", { name: "Codes (security key)" }));
  return { user, calls };
}

async function openWithKey(user: User) {
  await user.type(await screen.findByLabelText("Security key PIN"), "1234");
  await user.click(screen.getByRole("button", { name: "Use security key" }));
  await screen.findByRole("button", { name: "Lock this vault" });
}

/** The backup card's add-key dialog, filled and sent. The vault then asks for the master password. */
async function sendBackupKey(user: User) {
  const card = await screen.findByRole("region", { name: "Register a backup key" });
  await user.click(within(card).getByRole("button", { name: "Add security key" }));
  const dialog = await screen.findByRole("dialog", { name: "Add security key" });
  await user.type(within(dialog).getByLabelText("Security key PIN"), "1234");
  await user.click(within(dialog).getByRole("button", { name: "Add security key" }));
  return dialog;
}

async function confirmPassword(user: User, password: string) {
  const reauth = await screen.findByRole("dialog", { name: "Confirm with your master password" });
  await user.type(within(reauth).getByLabelText("Master password"), password);
  await user.click(within(reauth).getByRole("button", { name: "Confirm" }));
  return reauth;
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
    await openWithKey(user);
    await user.click(screen.getByRole("button", { name: "Lock this vault" }));
    expect(await screen.findByText("Locked. Your other vaults stay unlocked.")).toBeTruthy();
  });

  it("shows a wrong PIN as a sentence and keeps the panel", async () => {
    const { user } = await openHardwareVault();
    await user.type(await screen.findByLabelText("Security key PIN"), "0000");
    await user.click(screen.getByRole("button", { name: "Use security key" }));
    expect((await screen.findAllByRole("alert")).some((a) => /Wrong PIN/.test(a.textContent))).toBe(true);
    expect(screen.getByLabelText<HTMLInputElement>("Security key PIN").value).toBe("");
  });

  it("asks for the master password, then adds the backup key", async () => {
    const { user, calls } = await openHardwareVault();
    await openWithKey(user);
    const dialog = await sendBackupKey(user);

    // The first try went out without a password and was refused.
    const reauth = await confirmPassword(user, "wrong");
    expect(within(reauth).getByText(/Adding a key to this vault needs your master password/)).toBeTruthy();
    expect(await within(reauth).findByText(/didn't work/)).toBeTruthy();
    await user.type(within(reauth).getByLabelText("Master password"), "mosaic");
    await user.click(within(reauth).getByRole("button", { name: "Confirm" }));

    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Register a backup key" })).toBeNull();
    });
    expect(calls.filter((c) => c.command === "reauthenticate").map((c) => c.request)).toEqual([
      { password: "wrong" },
      { password: "mosaic" },
    ]);
    expect(calls.filter((c) => c.command === "add_hardware_vault_key")).toHaveLength(2);
    expect(dialog.isConnected).toBe(false);
    await user.click(await screen.findByRole("button", { name: /^Keys/ }));
    await screen.findByRole("dialog", { name: "Keys for this vault" });
  });

  it("leaves the add-key dialog open and quiet when the master password is cancelled", async () => {
    const { user, calls } = await openHardwareVault();
    await openWithKey(user);
    const dialog = await sendBackupKey(user);
    const reauth = await screen.findByRole("dialog", { name: "Confirm with your master password" });
    await user.click(within(reauth).getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: "Confirm with your master password" })).toBeNull();
    });
    expect(dialog.isConnected).toBe(true);
    expect(within(dialog).queryByText(/needs your master password|Cancelled/)).toBeNull();
    expect(within(dialog).getByRole<HTMLButtonElement>("button", { name: "Add security key" }).disabled).toBe(false);
    expect(calls.filter((c) => c.command === "reauthenticate")).toHaveLength(0);
  });

  it("asks for the master password to move an item out of the vault, then moves it", async () => {
    const { user, calls } = await openHardwareVault();
    await openWithKey(user);
    await user.click(await screen.findByText("Bank"));
    await user.click(await screen.findByRole("button", { name: "More actions" }));
    await user.click(screen.getByRole("menuitem", { name: /^Move to Personal/ }));

    const reauth = await screen.findByRole("dialog", { name: "Confirm with your master password" });
    expect(within(reauth).getByText(/Moving an item out of a vault protected by a security key/)).toBeTruthy();
    await confirmPassword(user, "mosaic");
    await waitFor(() => {
      expect(screen.queryByText("Bank")).toBeNull();
    });
    expect(calls.filter((c) => c.command === "move_item")).toHaveLength(2);
    expect(calls.some((c) => c.command === "reauthenticate")).toBe(true);
  });

  it("puts focus on Lock this vault after it opens", async () => {
    const { user } = await openHardwareVault();
    await openWithKey(user);
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByRole("button", { name: "Lock this vault" }));
    });
  });

  it("warns before the vault locks itself", async () => {
    const { user } = await openHardwareVault({ hwOpenSeconds: 20 });
    await openWithKey(user);
    expect(await screen.findByText(/locks in less than 30 seconds/)).toBeTruthy();
  });

  it("announces that the vault locked itself and focuses the PIN field", async () => {
    const { user } = await openHardwareVault({ hwOpenSeconds: 1 });
    await openWithKey(user);
    const notice = await screen.findByText("This vault locked itself. Use your security key to open it again.");
    expect(notice.getAttribute("role")).toBe("status");
    await waitFor(() => {
      expect(document.activeElement).toBe(screen.getByLabelText("Security key PIN"));
    });
  });

  it("offers a backup key only while one key is registered", async () => {
    const { user } = await openHardwareVault();
    await openWithKey(user);
    await sendBackupKey(user);
    await confirmPassword(user, "mosaic");
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Register a backup key" })).toBeNull();
    });
    await user.click(await screen.findByRole("button", { name: /^Keys/ }));
    await screen.findByRole("dialog", { name: "Keys for this vault" });
  });
});
