import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { openSettings } from "./testing.tsx";

async function openDialog(options: Parameters<typeof openSettings>[0] = {}) {
  const opened = await openSettings(options);
  await opened.user.click(screen.getByRole("button", { name: "Change…" }));
  const dialog = await screen.findByRole("dialog", { name: "Change master password" });
  return { ...opened, dialog };
}

// Whole-app renders and typed input are slow on a loaded machine.
vi.setConfig({ testTimeout: 60_000 });

describe("Change master password", () => {
  it("shows what changes, checks the two new passwords and changes the password", async () => {
    const { user, dialog, calls } = await openDialog({ sync: { variant: "hostedNoTotp" } });
    expect(within(dialog).getByText(/Your other devices are signed out of sync/)).toBeTruthy();
    expect(within(dialog).getByText(/Quick unlock on this device is turned off/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await user.type(within(dialog).getByLabelText("Current master password"), "mosaic");
    await user.type(within(dialog).getByLabelText("New master password"), "a better master password");
    await user.type(within(dialog).getByLabelText("Type the new master password again"), "a better master passwor");
    expect(within(dialog).getByText("The two passwords are different.")).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Change password" }).hasAttribute("disabled")).toBe(true);
    await user.type(within(dialog).getByLabelText("Type the new master password again"), "d");
    await user.clear(within(dialog).getByLabelText("Type the new master password again"));
    await user.type(within(dialog).getByLabelText("Type the new master password again"), "a better master password");
    await user.click(within(dialog).getByRole("button", { name: "Change password" }));
    expect(await within(dialog).findByRole("heading", { name: "Master password changed" })).toBeTruthy();
    expect(calls.filter((c) => c.command === "change_master_password")).toHaveLength(1);
    await user.click(within(dialog).getByRole("button", { name: "Close" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  it("refuses a wrong current password with the message and keeps the dialog open", async () => {
    const { user, dialog } = await openDialog();
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await user.type(within(dialog).getByLabelText("Current master password"), "nope");
    await user.type(within(dialog).getByLabelText("New master password"), "a better master password");
    await user.type(within(dialog).getByLabelText("Type the new master password again"), "a better master password");
    await user.click(within(dialog).getByRole("button", { name: "Change password" }));
    expect((await within(dialog).findAllByRole("alert")).some((a) => /didn't work/.test(a.textContent))).toBe(true);
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("asks for the second-factor code when the sync account has one", async () => {
    const { user, dialog } = await openDialog({ sync: { variant: "hosted" } });
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await user.type(within(dialog).getByLabelText("Current master password"), "mosaic");
    await user.type(within(dialog).getByLabelText("New master password"), "a better master password");
    await user.type(within(dialog).getByLabelText("Type the new master password again"), "a better master password");
    expect(within(dialog).getByRole("button", { name: "Change password" }).hasAttribute("disabled")).toBe(true);
    await user.type(within(dialog).getByLabelText("6-digit code from your authenticator app"), "123456");
    await user.click(within(dialog).getByRole("button", { name: "Change password" }));
    expect(await within(dialog).findByRole("heading", { name: "Master password changed" })).toBeTruthy();
  });
});
