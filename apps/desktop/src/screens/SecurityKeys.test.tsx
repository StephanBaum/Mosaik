import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../App.tsx";
import { createMockIpc } from "../ipc/mock.ts";
import { sampleAccount } from "../ipc/mock/data.ts";
import type { MockSyncOptions } from "../ipc/mock/syncData.ts";
import { POLL_MS } from "./sync/SecurityKeyStep.tsx";

/** The mock's browser step ends on its second status poll. */
async function polls(count: number) {
  for (let i = 0; i < count; i += 1) {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(POLL_MS + 50);
    });
  }
}

function newUser() {
  return userEvent.setup({ advanceTimers: (ms) => void vi.advanceTimersByTime(ms) });
}

async function openSync(sync: MockSyncOptions) {
  const user = newUser();
  render(<App ipc={createMockIpc({ start: "unlocked", sync })} languages={["en"]} />);
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: "Settings" }));
  const heading = await screen.findByRole("heading", { name: "Sync" });
  return { user, section: heading.closest("section") as HTMLElement };
}

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
});
afterEach(() => {
  vi.useRealTimers();
});

describe("the second-factor step with a security key", () => {
  it("signs in again with a key only: no code field, the code readable, the status announced", async () => {
    const { user, section } = await openSync({ variant: "signedOut", webauthn: "only" });
    await user.type(within(section).getByLabelText("Master password"), "mosaic");
    await user.click(within(section).getByRole("button", { name: "Sign in again" }));
    const heading = await within(section).findByRole("heading", { name: "Use your security key" });
    expect(within(section).queryByLabelText("6-digit code")).toBeNull();
    // Focus moves to the step's heading.
    await waitFor(() => {
      expect(document.activeElement).toBe(heading);
    });
    await user.click(within(section).getByRole("button", { name: "Use security key (opens your browser)" }));
    const step = await within(section).findByRole("heading", { name: "Sign in with your security key" });
    await waitFor(() => {
      expect(document.activeElement).toBe(step);
    });
    // The code is spoken digit by digit, and the status is a live region.
    expect(within(section).getByText("Code to type in your browser: 4 8 2 9 1 3")).toBeTruthy();
    expect(within(section).getByText("Waiting for your security key…")).toBeTruthy();
    expect(within(section).getByRole("button", { name: "Open the browser page again" })).toBeTruthy();
    await polls(2);
    await within(section).findByRole("button", { name: "Sync now" });
  });

  it("offers the key next to the code when the account has both", async () => {
    const { user, section } = await openSync({ variant: "signedOut", webauthn: "both" });
    await user.type(within(section).getByLabelText("Master password"), "mosaic");
    await user.click(within(section).getByRole("button", { name: "Sign in again" }));
    expect(await within(section).findByRole("button", { name: "Use security key (opens your browser)" })).toBeTruthy();
    expect(within(section).getByLabelText("6-digit code")).toBeTruthy();
    // The code still works.
    await user.type(within(section).getByLabelText("6-digit code"), "123456");
    await user.click(within(section).getByRole("button", { name: "Verify" }));
    await within(section).findByRole("button", { name: "Sync now" });
  });

  it("says when the browser step ran out of time and starts it again", async () => {
    const { user, section } = await openSync({ variant: "signedOut", webauthn: "only", handoff: "expired" });
    await user.type(within(section).getByLabelText("Master password"), "mosaic");
    await user.click(within(section).getByRole("button", { name: "Sign in again" }));
    await user.click(await within(section).findByRole("button", { name: "Use security key (opens your browser)" }));
    await polls(1);
    expect(await within(section).findByText("This step ran out of time. Start it again.")).toBeTruthy();
    expect(within(section).getByRole("button", { name: "Start again" })).toBeTruthy();
    // The code is gone with the step.
    expect(within(section).queryByText("Code to type in your browser")).toBeNull();
  });

  it("joins a new device with a security key", async () => {
    const user = newUser();
    render(<App ipc={createMockIpc({ sync: { webauthn: "only" } })} languages={["en"]} />);
    await user.click(await screen.findByRole("button", { name: "I already use Mosaic" }));
    await user.type(await screen.findByLabelText("Server address"), "https://sync.example.com");
    await user.type(screen.getByLabelText("Sync account email"), "alex@example.com");
    await user.type(screen.getByLabelText("Master password"), "mosaic");
    await user.type(screen.getByLabelText("Secret Key"), sampleAccount.secretKey);
    await user.click(screen.getByRole("button", { name: "Add this device" }));
    await screen.findByRole("heading", { name: "Use your security key" });
    expect(screen.queryByLabelText("6-digit code")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Use security key (opens your browser)" }));
    await screen.findByText("Code to type in your browser: 4 8 2 9 1 3");
    await polls(2);
    expect(await screen.findByText("GitHub")).toBeTruthy();
  });
});

describe("the security key list", () => {
  it("lists the keys and adds one after the master password and a code", async () => {
    const { user, section } = await openSync({ variant: "hosted", webauthn: "both" });
    expect(await within(section).findByText("YubiKey 5C")).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: "Add a security key…" }));
    const dialog = within(screen.getByRole("dialog"));
    // A code backup exists, so no warning about the only second step.
    expect(dialog.queryByText(/would be your only second step/)).toBeNull();
    await user.type(dialog.getByLabelText("Name of the key"), "Desk key");
    await user.type(dialog.getByLabelText("Master password"), "mosaic");
    await user.type(dialog.getByLabelText("6-digit code"), "123456");
    await user.click(dialog.getByRole("button", { name: "Continue" }));
    await dialog.findByRole("heading", { name: "Add your security key" });
    await polls(2);
    expect(await within(section).findByText("Desk key")).toBeTruthy();
    expect(await screen.findByText("Security key added.")).toBeTruthy();
  });

  it("goes through the sign-in step first when no code is given", async () => {
    const { user, section } = await openSync({ variant: "hosted", webauthn: "only" });
    await within(section).findByText("YubiKey 5C");
    await user.click(within(section).getByRole("button", { name: "Add a security key…" }));
    const dialog = within(screen.getByRole("dialog"));
    await user.type(dialog.getByLabelText("Name of the key"), "Backup");
    await user.type(dialog.getByLabelText("Master password"), "mosaic");
    await user.click(dialog.getByRole("button", { name: "Continue" }));
    await dialog.findByRole("heading", { name: "Sign in with your security key" });
    await polls(2);
    // The sign-in finished: the add step follows with its own code.
    await dialog.findByRole("heading", { name: "Add your security key" });
    await polls(2);
    expect(await within(section).findByText("Backup")).toBeTruthy();
  });

  it("warns that a first key without a code backup is the only second step", async () => {
    const { user, section } = await openSync({ variant: "hostedNoTotp" });
    await within(section).findByText("No security keys added.");
    await user.click(within(section).getByRole("button", { name: "Add a security key…" }));
    expect(within(screen.getByRole("dialog")).getByText(/would be your only second step/)).toBeTruthy();
  });

  it("recommends two methods and warns before the last one goes", async () => {
    const { user, section } = await openSync({ variant: "hosted", webauthn: "only" });
    await within(section).findByText("YubiKey 5C");
    expect(within(section).getByText(/We recommend two methods/)).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: "Remove YubiKey 5C" }));
    const dialog = within(screen.getByRole("dialog"));
    expect(dialog.getByText(/Two-step verification turns off/)).toBeTruthy();
    await user.click(dialog.getByRole("button", { name: "Remove key" }));
    // No recent confirm: the master password and a key in the browser come first.
    await dialog.findByText(/Removing a key needs a second step/);
    await user.type(dialog.getByLabelText("Master password"), "mosaic");
    await user.click(dialog.getByRole("button", { name: "Continue" }));
    await dialog.findByRole("heading", { name: "Sign in with your security key" });
    await polls(2);
    await dialog.findByRole("heading", { name: "Confirm with your security key" });
    await polls(2);
    await waitFor(() => {
      expect(within(section).queryByText("YubiKey 5C")).toBeNull();
    });
    expect(await screen.findByText("Security key removed.")).toBeTruthy();
  });
});
