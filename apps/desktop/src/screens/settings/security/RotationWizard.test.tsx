import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { openSettings } from "./testing.tsx";

type User = ReturnType<typeof userEvent.setup>;

async function start(user: User, name: string) {
  await user.click(screen.getByRole("button", { name }));
  return screen.findByRole("dialog", { name });
}

async function heading(dialog: HTMLElement, name: string | RegExp) {
  const found = await within(dialog).findByRole("heading", { name });
  await waitFor(() => {
    expect(document.activeElement).toBe(found);
  });
  return found;
}

async function typePassword(user: User, dialog: HTMLElement, label: string, text: string) {
  await user.type(within(dialog).getByLabelText(label), text);
}

// Whole-app renders and typed input are slow on a loaded machine.
vi.setConfig({ testTimeout: 60_000 });

describe("Rotate Secret Key wizard", () => {
  it("shows the review, saves and confirms the new kit, then rotates", async () => {
    const { user, calls } = await openSettings({ sync: { variant: "folder" } });
    const dialog = await start(user, "Rotate Secret Key");
    await heading(dialog, "Before you rotate your Secret Key");
    expect(within(dialog).getByText(/This stops sign-in with the old Secret Key/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await heading(dialog, "Your passwords");
    await typePassword(user, dialog, "Current master password", "wrong");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    expect((await within(dialog).findAllByRole("alert")).some((a) => /didn't work/.test(a.textContent))).toBe(true);
    await user.clear(within(dialog).getByLabelText("Current master password"));
    await typePassword(user, dialog, "Current master password", "mosaic");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));

    // The kit comes before the backend learns anything.
    await heading(dialog, "Save your new Emergency Kit");
    expect(calls.some((c) => c.command === "secret_key_rotate_commit")).toBe(false);
    await user.click(within(dialog).getByRole("button", { name: "Save new Emergency Kit" }));
    await user.type(await within(dialog).findByLabelText(/last group of your new Secret Key/), "ZZZZ");
    await user.click(within(dialog).getByRole("button", { name: "Confirm kit" }));
    expect(await within(dialog).findByText(/not the end of the new Secret Key/)).toBeTruthy();
    await user.clear(within(dialog).getByLabelText(/last group of your new Secret Key/));
    await user.type(within(dialog).getByLabelText(/last group of your new Secret Key/), "8vj3");
    await user.click(within(dialog).getByRole("button", { name: "Confirm kit" }));

    await heading(dialog, "Rotate your Secret Key");
    await user.click(within(dialog).getByRole("button", { name: "Rotate Secret Key" }));
    await heading(dialog, "Secret Key rotated");
    expect(calls.map((c) => c.command)).toContain("secret_key_rotate_commit");
    await user.click(within(dialog).getByRole("button", { name: "Close" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  it("offers the re-key when something was compromised", async () => {
    const { user } = await openSettings({ sync: { variant: "folder" } });
    const dialog = await start(user, "Rotate Secret Key");
    await user.click(within(dialog).getByRole("button", { name: "Something was compromised: re-key instead" }));
    await heading(dialog, "Was something compromised?");
  });

  it("discards what it staged when it is closed before the run", async () => {
    const { user, calls } = await openSettings({ sync: { variant: "folder" } });
    const dialog = await start(user, "Rotate Secret Key");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await typePassword(user, dialog, "Current master password", "mosaic");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await heading(dialog, "Save your new Emergency Kit");
    fireEvent(dialog, new Event("cancel", { cancelable: true }));
    await waitFor(() => {
      expect(calls.map((c) => c.command)).toContain("rotation_abandon");
    });
  });
});

describe("Re-key account wizard", () => {
  it("asks first, reviews what it touches and runs with phases", async () => {
    const { user, calls } = await openSettings({ rotation: { variant: "shared", stepMs: 0 }, sync: { variant: "hostedNoTotp" } });
    const dialog = await start(user, "Re-key account");
    await heading(dialog, "Was something compromised?");
    await user.click(within(dialog).getByRole("button", { name: "Yes, re-key my account" }));
    await heading(dialog, "Your master passwords");
    await typePassword(user, dialog, "Current master password", "mosaic");
    await typePassword(user, dialog, "New master password", "a new master password");
    await typePassword(user, dialog, "Type the new master password again", "a different one");
    expect(within(dialog).getByText("The two passwords are different.")).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Continue" }).hasAttribute("disabled")).toBe(true);
    await user.clear(within(dialog).getByLabelText("Type the new master password again"));
    await typePassword(user, dialog, "Type the new master password again", "a new master password");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));

    await heading(dialog, "Save your new Emergency Kit");
    await user.click(within(dialog).getByRole("button", { name: "Save new Emergency Kit" }));
    await user.type(await within(dialog).findByLabelText(/last group of your new Secret Key/), "8VJ3");
    await user.click(within(dialog).getByRole("button", { name: "Confirm kit" }));

    await heading(dialog, "Re-key your account");
    expect(await within(dialog).findByText(/Ask each person you share with to compare your new safety code/)).toBeTruthy();
    expect(within(dialog).getByText(/Family/)).toBeTruthy();
    expect(within(dialog).getByText(/does not take back what someone already copied/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Re-key account" }));
    await heading(dialog, "Account re-keyed");
    expect(within(dialog).getByText(/Ask everyone you share a vault with/)).toBeTruthy();
    expect(calls.map((c) => c.command)).toContain("rekey_commit");
  });

  it("says why it cannot run without sync", async () => {
    await openSettings({ sync: { variant: "none" } });
    expect(screen.getByText(/needs sync/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Re-key account" }).hasAttribute("disabled")).toBe(true);
  });

  it("shows the error and stays in the wizard when the run fails", async () => {
    const { user } = await openSettings({ rotation: { variant: "none", stepMs: 0, fail: "rekeyNeedsCleanSync" } , sync: { variant: "folder" } });
    const dialog = await start(user, "Re-key account");
    await user.click(within(dialog).getByRole("button", { name: "Yes, re-key my account" }));
    await typePassword(user, dialog, "Current master password", "mosaic");
    await typePassword(user, dialog, "New master password", "a new master password");
    await typePassword(user, dialog, "Type the new master password again", "a new master password");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await user.click(await within(dialog).findByRole("button", { name: "Save new Emergency Kit" }));
    await user.type(await within(dialog).findByLabelText(/last group of your new Secret Key/), "8VJ3");
    await user.click(within(dialog).getByRole("button", { name: "Confirm kit" }));
    await user.click(await within(dialog).findByRole("button", { name: "Re-key account" }));
    expect(await within(dialog).findByText(/needs sync to be up to date/)).toBeTruthy();
  });
});

describe("Upgrade protection wizard", () => {
  it("warns on a slow device and goes on when asked", async () => {
    const { user } = await openSettings({ rotation: { variant: "slowDevice", stepMs: 0 }, sync: { variant: "hostedNoTotp" } });
    const dialog = await start(user, "Upgrade protection");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await typePassword(user, dialog, "Current master password", "mosaic");
    await user.click(within(dialog).getByRole("button", { name: "Upgrade protection" }));
    expect(await within(dialog).findByText(/takes this device more than two seconds/)).toBeTruthy();
    await user.click(within(dialog).getByRole("switch", { name: "Upgrade anyway" }));
    await user.click(within(dialog).getByRole("button", { name: "Upgrade protection" }));
    await heading(dialog, "Protection upgraded");
  });
});

describe("German", () => {
  it("shows the wizard in German without the word epoch", async () => {
    const { user } = await openSettings({ sync: { variant: "folder" } }, "de");
    const dialog = await start(user, "Konto neu verschlüsseln");
    await heading(dialog, "Wurde etwas kompromittiert?");
    expect(dialog.textContent).not.toMatch(/epoch/i);
    expect(within(dialog).getByText(/Ein Neuverschlüsseln ist für den Fall gedacht/)).toBeTruthy();
  });
});
