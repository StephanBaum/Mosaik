import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "../../../App.tsx";
import { createMockIpc, type MockOptions } from "../../../ipc/mock.ts";
import { MOCK_NEW_SECRET_KEY } from "../../../ipc/mock/rotation.ts";
import { MOCK_NEW_PASSWORD } from "../../../ipc/mock/syncData.ts";
import { openSettings } from "./testing.tsx";

async function openApp(options: MockOptions) {
  const user = userEvent.setup();
  render(<App ipc={createMockIpc({ start: "unlocked", rotation: { stepMs: 0 }, ...options })} languages={["en"]} />);
  await screen.findByText("GitHub");
  return user;
}

const RECOVERY_PASSWORD = "Master password (the new one, if you changed it there)";

// Whole-app renders and typed input are slow on a loaded machine.
vi.setConfig({ testTimeout: 60_000 });

describe("another device renewed the keys", () => {
  it("shows an in-app notice and adopts with the new Secret Key", async () => {
    const user = await openApp({ rotation: { variant: "rekeyPending", stepMs: 0 } });
    const notice = await screen.findByRole("region", { name: "Your account's keys were renewed on another device" });
    expect(within(notice).getByText(/Sync is paused/)).toBeTruthy();
    await user.click(within(notice).getByRole("button", { name: "Use the new keys…" }));
    const dialog = await screen.findByRole("dialog", { name: "Your account's keys were renewed on another device" });
    // A key this device used is refused with its own message.
    await user.type(within(dialog).getByLabelText(RECOVERY_PASSWORD), MOCK_NEW_PASSWORD);
    await user.type(within(dialog).getByLabelText("Secret Key from the new Emergency Kit"), "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2");
    await user.click(within(dialog).getByRole("button", { name: "Use the new keys" }));
    expect(await within(dialog).findByText(/no longer works/)).toBeTruthy();
    await user.clear(within(dialog).getByLabelText(RECOVERY_PASSWORD));
    await user.clear(within(dialog).getByLabelText("Secret Key from the new Emergency Kit"));
    await user.type(within(dialog).getByLabelText(RECOVERY_PASSWORD), MOCK_NEW_PASSWORD);
    await user.type(within(dialog).getByLabelText("Secret Key from the new Emergency Kit"), MOCK_NEW_SECRET_KEY);
    await user.click(within(dialog).getByRole("button", { name: "Use the new keys" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Your account's keys were renewed on another device" })).toBeNull();
    });
  });

  it("asks for the new Secret Key, not a sign-in, after a hosted re-key elsewhere", async () => {
    const { user } = await openSettings({ rotation: { variant: "rekeyElsewhere", stepMs: 0 } });
    const field = await screen.findByLabelText("Secret Key from the new Emergency Kit");
    // No "password changed" card with an optional key, and no second sign-in form beside it.
    expect(screen.queryByText(/Your vaults and your Secret Key stay the same/)).toBeNull();
    expect(screen.queryByLabelText("Secret Key from the newest Emergency Kit (only if you renewed it)")).toBeNull();
    expect(screen.queryByRole("heading", { name: "This device is signed out" })).toBeNull();
    await user.type(screen.getByLabelText(RECOVERY_PASSWORD), MOCK_NEW_PASSWORD);
    await user.type(field, MOCK_NEW_SECRET_KEY);
    await user.click(screen.getByRole("button", { name: "Use the new keys" }));
    await waitFor(() => {
      expect(screen.queryByLabelText("Secret Key from the new Emergency Kit")).toBeNull();
    });
  });
});

describe("folder findings", () => {
  it("dismisses an unknown account file", async () => {
    const { user } = await openSettings({ rotation: { variant: "unknownFile", stepMs: 0 } });
    expect(await screen.findByText(/An unknown account file appeared in your sync folder/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    await waitFor(() => {
      expect(screen.queryByText(/An unknown account file appeared/)).toBeNull();
    });
  });

  it("explains a concurrent re-key and offers to re-key again", async () => {
    const { user } = await openSettings({ rotation: { variant: "conflict", stepMs: 0 } });
    expect(await screen.findByText("Two devices renewed the keys at the same time")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Re-key again from this device" }));
    expect(await screen.findByRole("dialog", { name: "Re-key account" })).toBeTruthy();
  });
});

describe("continue key rotation", () => {
  it("shows the banner and continues a job that needs nothing", async () => {
    const user = await openApp({ rotation: { variant: "resumeAttachments", stepMs: 0 } });
    const banner = await screen.findByRole("region", { name: "Key change not finished" });
    await user.click(within(banner).getByRole("button", { name: "Continue key rotation" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Key change not finished" })).toBeNull();
    });
  });

  it("asks for the new password and the last kit group when the account may have changed", async () => {
    const user = await openApp({ rotation: { variant: "resumeAccepted", stepMs: 0 } });
    const banner = await screen.findByRole("region", { name: "Key change not finished" });
    await user.click(within(banner).getByRole("button", { name: "Continue key rotation" }));
    const dialog = await screen.findByRole("dialog", { name: "Continue key rotation" });
    await user.type(within(dialog).getByLabelText("Master password you have now"), "x");
    await user.type(within(dialog).getByLabelText("Last group of the new Secret Key"), "ZZZZ");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    expect(await within(dialog).findByText(/not the last group of the new Secret Key/)).toBeTruthy();
    await user.type(within(dialog).getByLabelText("Master password you have now"), "x");
    await user.type(within(dialog).getByLabelText("Last group of the new Secret Key"), "8VJ3");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Key change not finished" })).toBeNull();
    });
  });

  it("discards a job that never reached the account, but not one that did", async () => {
    const user = await openApp({ rotation: { variant: "resumeAccepted", stepMs: 0 } });
    const banner = await screen.findByRole("region", { name: "Key change not finished" });
    await user.click(within(banner).getByRole("button", { name: "Discard…" }));
    const dialog = await screen.findByRole("dialog", { name: "Discard this key change?" });
    await user.click(within(dialog).getByRole("button", { name: "Discard key change" }));
    expect(await within(dialog).findByText(/already reached your account/)).toBeTruthy();
  });
});
