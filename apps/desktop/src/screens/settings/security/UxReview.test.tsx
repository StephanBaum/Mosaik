/**
 * The M7 UX review follow-ups: visible progress, focus that never falls to the page, the stop
 * confirmation, the re-key conflict and the announcements.
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { I18nProvider } from "../../../i18n/I18nProvider.tsx";
import { RunProgress } from "./RunProgress.tsx";
import { openSettings } from "./testing.tsx";

// Whole-app renders and typed input are slow on a loaded machine.
vi.setConfig({ testTimeout: 60_000 });

const RECOVERY = "Your account's keys were renewed on another device";

function focusIsOnThePage(): boolean {
  return document.activeElement !== document.body && document.activeElement !== null;
}

describe("the run progress", () => {
  it("shows the phase name and list, not inside the thin setup bar", () => {
    const { container } = render(
      <I18nProvider locale="en">
        <RunProgress
          job={{ kind: "rekey", phase: "attachments", done: 3, total: 12, needs: "none" }}
          backend="hosted"
        />
      </I18nProvider>,
    );
    const root = container.firstElementChild;
    // `.progress` (app.css) is an 8 px bar with hidden overflow: the phases must not live in it.
    expect(root?.className).toBe("rotation-progress");
    expect(container.querySelector(".progress")).toBeNull();
    expect(within(container).getByRole("status").textContent).toMatch(/\S/);
    expect(within(container).getByText(/3 of 12/)).toBeTruthy();
  });
});

describe("focus after a control is replaced or removed", () => {
  it("moves to the verify field after Save new Emergency Kit", async () => {
    const { user } = await openSettings({ sync: { variant: "folder" } });
    await user.click(screen.getByRole("button", { name: "Rotate…" }));
    const dialog = await screen.findByRole("dialog", { name: "Rotate Secret Key" });
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await user.type(within(dialog).getByLabelText("Current master password"), "mosaic");
    await user.click(within(dialog).getByRole("button", { name: "Continue" }));
    await user.click(await within(dialog).findByRole("button", { name: "Save new Emergency Kit" }));
    const field = await within(dialog).findByLabelText(/last group of your new Secret Key/);
    await waitFor(() => {
      expect(document.activeElement).toBe(field);
    });
    // One action row: Back and Confirm kit together.
    const confirm = within(dialog).getByRole("button", { name: "Confirm kit" });
    expect(within(dialog).getByRole("button", { name: "Back" }).parentElement).toBe(confirm.parentElement);
  });

  it("stays on the page when Continue key rotation finishes and the banner goes", async () => {
    const { user } = await openSettings({ rotation: { variant: "resumeAttachments", stepMs: 0 } });
    const banner = await screen.findByRole("region", { name: "Key change not finished" });
    await user.click(within(banner).getByRole("button", { name: "Continue key rotation" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Key change not finished" })).toBeNull();
    });
    await waitFor(() => {
      expect(focusIsOnThePage()).toBe(true);
    });
  });

  it("stays on the page after Ignore notice", async () => {
    const { user } = await openSettings({ rotation: { variant: "rekeyPending", stepMs: 0 } });
    const notice = await screen.findByRole("region", { name: RECOVERY });
    await user.click(await within(notice).findByRole("button", { name: /^Ignore this notice/ }));
    const dialog = await screen.findByRole("dialog", { name: "Unconfirmed re-key notice" });
    await user.click(within(dialog).getByRole("button", { name: "Ignore notice" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: RECOVERY })).toBeNull();
    });
    await waitFor(() => {
      expect(focusIsOnThePage()).toBe(true);
    });
  });

  it("stays on the page after Restore", async () => {
    const { user } = await openSettings({ rotation: { variant: "ignoredNotices", stepMs: 0 } });
    const section = await screen.findByRole("region", { name: "Ignored re-key notices" });
    await user.click(within(section).getByRole("button", { name: /Restore the notice/ }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Ignored re-key notices" })).toBeNull();
    });
    await waitFor(() => {
      expect(focusIsOnThePage()).toBe(true);
    });
  });
});

describe("the stop confirmation", () => {
  it("closes on Esc and returns to the wizard, not to the same question", async () => {
    const { user } = await openSettings({ sync: { variant: "folder" } });
    await user.click(screen.getByRole("button", { name: "Rotate…" }));
    const wizard = await screen.findByRole("dialog", { name: "Rotate Secret Key" });
    await user.click(within(wizard).getByRole("button", { name: "Continue" }));
    await user.type(within(wizard).getByLabelText("Current master password"), "mosaic");
    await user.click(within(wizard).getByRole("button", { name: "Continue" }));
    await within(wizard).findByRole("button", { name: "Save new Emergency Kit" });
    fireEvent(wizard, new Event("cancel", { cancelable: true }));
    const stop = await screen.findByRole("dialog", { name: "Stop this change?" });
    fireEvent(stop, new Event("cancel", { bubbles: true, cancelable: true }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: "Stop this change?" })).toBeNull();
    });
    expect(screen.getByRole("dialog", { name: "Rotate Secret Key" })).toBeTruthy();
  });
});

describe("announcements", () => {
  it("keeps the resume banner title in a status region", async () => {
    await openSettings({ rotation: { variant: "resumeAttachments", stepMs: 0 } });
    await screen.findByRole("region", { name: "Key change not finished" });
    const statuses = screen.getAllByRole("status").map((s) => s.textContent);
    expect(statuses).toContain("Key change not finished");
  });
});

describe("two devices that re-keyed at once", () => {
  it("does not read Synced and opens the re-key at the passwords", async () => {
    const { user } = await openSettings({ rotation: { variant: "conflict", stepMs: 0 } });
    expect((await screen.findAllByText("Needs attention")).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "Re-key again from this device" }));
    const dialog = await screen.findByRole("dialog", { name: "Re-key account" });
    // Not "Was something compromised?": that question was already answered.
    expect(await within(dialog).findByRole("heading", { name: "Your master passwords" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Hide notice" })).toBeTruthy();
  });
});
