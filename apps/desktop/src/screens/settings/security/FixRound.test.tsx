/** The M7 fix round in the UI: ignoring a re-key notice and giving up a committed key job. */
import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { openSettings } from "./testing.tsx";

// Whole-app renders are slow on a loaded machine.
vi.setConfig({ testTimeout: 60_000 });

describe("an unconfirmed re-key notice", () => {
  it("is ignored only after a confirmation, and the halt clears", async () => {
    const { user, calls } = await openSettings({ rotation: { variant: "rekeyPending", stepMs: 0 } });
    const notice = await screen.findByRole("region", { name: "Your account's keys were renewed on another device" });
    await user.click(await within(notice).findByRole("button", { name: "Ignore this notice…" }));
    const dialog = await screen.findByRole("dialog", { name: "Unconfirmed re-key notice" });
    expect(
      within(dialog).getByText(
        "Mosaic found a re-key notice it couldn't confirm. If you didn't re-key your account on another device, you can ignore it.",
      ),
    ).toBeTruthy();
    // "Keep" leaves everything as it is.
    await user.click(within(dialog).getByRole("button", { name: "Keep" }));
    expect(screen.queryByRole("dialog", { name: "Unconfirmed re-key notice" })).toBeNull();
    expect(calls.map((c) => c.command)).not.toContain("sync_dismiss_rekey_marker");
    await user.click(within(notice).getByRole("button", { name: "Ignore this notice…" }));
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
