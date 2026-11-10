/** The M7 fix round in the UI: ignoring a re-key notice and giving up a committed key job. */
import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { openSettings } from "./testing.tsx";

const RECOVERY_PASSWORD = "Master password (the new one, if you changed it there)";

// Whole-app renders are slow on a loaded machine.
vi.setConfig({ testTimeout: 60_000 });

describe("an unconfirmed re-key notice", () => {
  it("is ignored only after a confirmation, and the halt clears", async () => {
    const { user, calls } = await openSettings({ rotation: { variant: "rekeyPending", stepMs: 0 } });
    const notice = await screen.findByRole("region", { name: "Your account's keys were renewed on another device" });
    await user.click(await within(notice).findByRole("button", { name: /^Ignore this notice/ }));
    const dialog = await screen.findByRole("dialog", { name: "Unconfirmed re-key notice" });
    expect(
      within(dialog).getByText(
        "Mosaic found a re-key notice it couldn't confirm. If you didn't re-key your account on another device, you can ignore it.",
      ),
    ).toBeTruthy();
    // "Keep" leaves everything as it is.
    await user.click(within(dialog).getByRole("button", { name: "Keep notice" }));
    expect(screen.queryByRole("dialog", { name: "Unconfirmed re-key notice" })).toBeNull();
    expect(calls.map((c) => c.command)).not.toContain("sync_dismiss_rekey_marker");
    await user.click(within(notice).getByRole("button", { name: /^Ignore this notice/ }));
    const again = await screen.findByRole("dialog", { name: "Unconfirmed re-key notice" });
    await user.click(within(again).getByRole("button", { name: "Ignore notice" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Your account's keys were renewed on another device" })).toBeNull();
    });
    expect(calls.map((c) => c.command)).toContain("sync_dismiss_rekey_marker");
  });
});

describe("a committed key job that cannot finish", () => {
  it("offers to give up only after a continue did not finish, and asks first", async () => {
    const { user, calls } = await openSettings({ rotation: { variant: "resumeStuck", stepMs: 0 } });
    const banner = await screen.findByRole("region", { name: "Key change not finished" });
    expect(within(banner).queryByRole("button", { name: "Give up on the rest…" })).toBeNull();
    await user.click(within(banner).getByRole("button", { name: "Continue key rotation" }));
    await user.click(await within(banner).findByRole("button", { name: "Give up on the rest…" }));
    const dialog = await screen.findByRole("dialog", { name: "Stop re-encrypting your files?" });
    expect(within(dialog).getByText(/keep their old file key/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Keep trying" }));
    expect(calls.map((c) => c.command)).not.toContain("rotation_abandon_committed");
    await user.click(within(banner).getByRole("button", { name: "Give up on the rest…" }));
    const confirm = await screen.findByRole("dialog", { name: "Stop re-encrypting your files?" });
    await user.click(within(confirm).getByRole("button", { name: "Stop re-encrypting" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Key change not finished" })).toBeNull();
    });
    expect(calls.map((c) => c.command)).toContain("rotation_abandon_committed");
  });
});

describe("ignored re-key notices", () => {
  it("are listed only when there are some, and Restore halts sync again without asking", async () => {
    const { user, calls } = await openSettings({ rotation: { variant: "ignoredNotices", stepMs: 0 } });
    const section = await screen.findByRole("region", { name: "Ignored re-key notices" });
    expect(screen.queryByRole("region", { name: "Your account's keys were renewed on another device" })).toBeNull();
    await user.click(within(section).getByRole("button", { name: /Restore the notice/ }));
    expect(await screen.findByRole("region", { name: "Your account's keys were renewed on another device" })).toBeTruthy();
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Ignored re-key notices" })).toBeNull();
    });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(calls.map((c) => c.command)).toContain("sync_restore_rekey_marker");
  });

  it("do not show the section when nothing was ignored", async () => {
    await openSettings({ rotation: { variant: "rekeyPending", stepMs: 0 } });
    expect(screen.queryByRole("region", { name: "Ignored re-key notices" })).toBeNull();
  });
});

describe("a device too far behind", () => {
  it("is told to join again and is not offered a re-key", async () => {
    const { user } = await openSettings({ rotation: { variant: "jumpTooLarge", stepMs: 0 } });
    const notice = await screen.findByRole("region", { name: "Your account's keys were renewed on another device" });
    await user.click(within(notice).getByRole("button", { name: "Use the new keys…" }));
    const dialog = await screen.findByRole("dialog", { name: "Your account's keys were renewed on another device" });
    await user.type(within(dialog).getByLabelText(RECOVERY_PASSWORD), "whatever password");
    await user.type(within(dialog).getByLabelText("Secret Key from the new Emergency Kit"), "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ3");
    await user.click(within(dialog).getByRole("button", { name: "Use the new keys" }));
    expect(await within(dialog).findByText(/too far behind the other devices to catch up\. Set it up again/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Re-key again/ })).toBeNull();
  });
});
