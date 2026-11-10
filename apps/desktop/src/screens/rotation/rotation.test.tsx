/**
 * The rotate-now queue, the guided panel, the item banner and history, and the breach card
 * (M7b), on the mock with its `credRotation` variants: focus on the step headings, the plain
 * consent text, "Not now" sending nothing, the three outcomes and the polite status.
 */
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "../../App.tsx";
import type { MockOptions } from "../../ipc/mock.ts";
import { recording } from "../settings/security/testing.tsx";

vi.setConfig({ testTimeout: 60_000 });

type User = ReturnType<typeof userEvent.setup>;

async function openRotate(options: MockOptions, language = "en") {
  const { ipc, calls } = recording({ start: "unlocked", credRotation: { stepMs: 0 }, ...options });
  const user = userEvent.setup();
  render(<App ipc={ipc} languages={[language]} />);
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: /^(Security|Sicherheit)/ }));
  await user.click(await screen.findByRole("tab", { name: /^(Rotate now|Jetzt ändern)/ }));
  return { user, calls, ipc };
}

async function heading(dialog: HTMLElement, name: string | RegExp) {
  const found = await within(dialog).findByRole("heading", { name });
  await waitFor(() => {
    expect(document.activeElement).toBe(found);
  });
  return found;
}

async function startChange(user: User, title: string) {
  await user.click(await screen.findByRole("button", { name: `Change password: ${title}` }));
  return screen.findByRole("dialog", { name: `Change password for ${title}` });
}

describe("rotate-now queue", () => {
  it("lists what to change first, with reasons, and says what it can't guide", async () => {
    await openRotate({ credRotation: { variant: "queue", stepMs: 0 } });
    const list = await screen.findByRole("list", { name: "Passwords to change" });
    expect(within(list).getAllByRole("listitem").length).toBeGreaterThanOrEqual(3);
    expect(within(list).getAllByText("In a breach").length).toBeGreaterThan(0);
    expect(within(list).getAllByText("Weak").length).toBeGreaterThan(0);
    // The Wi-Fi password has no https page: "Edit item", with a hint.
    expect(within(list).getByRole("button", { name: "Edit item: Wi-Fi at home" })).toBeTruthy();
    expect(within(list).getByText(/can't guide this one/)).toBeTruthy();
    // The tab names its count.
    expect(screen.getByRole("tab", { name: /Rotate now, \d+ to change/ })).toBeTruthy();
  });

  it("says so when there is nothing to change", async () => {
    await openRotate({ credRotation: { variant: "empty", stepMs: 0 } });
    expect(await screen.findByText("Nothing to change right now.")).toBeTruthy();
  });

  it("snoozes an item (listed apart) and dismisses another", async () => {
    const { user, calls } = await openRotate({ credRotation: { variant: "queue", stepMs: 0 } });
    await user.click(await screen.findByRole("button", { name: "Snooze Figma" }));
    await user.click(await screen.findByRole("menuitem", { name: "7 days" }));
    expect(await screen.findByRole("heading", { name: "Snoozed" })).toBeTruthy();
    expect(calls.some((c) => c.command === "rotation_snooze" && JSON.stringify(c.request).includes('"days":7'))).toBe(true);
    await user.click(await screen.findByRole("button", { name: "Dismiss Jira" }));
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "Dismiss Jira" })).toBeNull();
    });
  });

  it("speaks German too", async () => {
    await openRotate({ credRotation: { variant: "queue", stepMs: 0 } }, "de");
    expect(await screen.findByRole("list", { name: "Passwörter, die du ändern solltest" })).toBeTruthy();
    expect(screen.getAllByText("In einem Datenleck").length).toBeGreaterThan(0);
  });
});

describe("guided change panel", () => {
  it("stores the new password before the site step, then confirms: it worked", async () => {
    const { user, calls } = await openRotate({ credRotation: { variant: "queue", stepMs: 0 } });
    const dialog = await startChange(user, "Figma");
    await heading(dialog, "Choose a new password");
    expect(calls.some((c) => c.command === "rotation_begin")).toBe(false);
    await user.click(await within(dialog).findByRole("button", { name: "Use this password" }));

    await heading(dialog, "Change it on the site");
    expect(calls.filter((c) => c.command === "rotation_begin")).toHaveLength(1);
    expect(within(dialog).getByText(/Mosaic never submits the form/)).toBeTruthy();
    expect(within(dialog).getByDisplayValue("https://www.figma.com/.well-known/change-password")).toBeTruthy();
    // The password is copied through Rust, the link as plain text; nothing opens a page.
    await user.click(within(dialog).getByRole("button", { name: "Copy new password" }));
    expect(calls.some((c) => c.command === "rotation_copy")).toBe(true);
    await user.click(within(dialog).getByRole("button", { name: "I've tried it" }));

    await heading(dialog, "Did the site accept the new password?");
    expect(calls.some((c) => c.command === "rotation_close")).toBe(false);
    await user.click(within(dialog).getByRole("button", { name: "It worked" }));
    await heading(dialog, "Done");
    expect(within(dialog).getByRole("status").textContent).toMatch(/new password is now this item's password/);
    expect(JSON.stringify(calls.find((c) => c.command === "rotation_close")?.request)).toContain('"outcome":"succeeded"');
    await user.click(within(dialog).getByRole("button", { name: "Done" }));
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("keeps the change open on Not sure yet and shows it as unfinished", async () => {
    const { user, calls } = await openRotate({ credRotation: { variant: "queue", stepMs: 0 } });
    const dialog = await startChange(user, "Figma");
    await user.click(await within(dialog).findByRole("button", { name: "Use this password" }));
    await user.click(await within(dialog).findByRole("button", { name: "I've tried it" }));
    await user.click(await within(dialog).findByRole("button", { name: "Not sure yet" }));
    expect(within(dialog).getByRole("status").textContent).toMatch(/stays open on this item/);
    expect(calls.some((c) => c.command === "rotation_close")).toBe(false);
    await user.click(within(dialog).getByRole("button", { name: "Done" }));
    expect(await screen.findByRole("button", { name: "Continue change: Figma" })).toBeTruthy();
    expect(screen.getAllByText("Change not finished").length).toBeGreaterThan(0);
  });

  it("keeps the current password when it didn't work, and the value in the history", async () => {
    const { user, calls } = await openRotate({ credRotation: { variant: "queue", stepMs: 0 } });
    const dialog = await startChange(user, "Figma");
    await user.click(await within(dialog).findByRole("button", { name: "Use this password" }));
    await user.click(await within(dialog).findByRole("button", { name: "I've tried it" }));
    await user.click(await within(dialog).findByRole("button", { name: "It didn't work" }));
    expect(within(dialog).getByRole("status").textContent).toMatch(/current password stays/);
    expect(JSON.stringify(calls.find((c) => c.command === "rotation_close")?.request)).toContain('"outcome":"failed"');
  });

  it("closes without a trace when cancelled at the first step", async () => {
    const { user, calls } = await openRotate({ credRotation: { variant: "queue", stepMs: 0 } });
    const dialog = await startChange(user, "Figma");
    await user.click(await within(dialog).findByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(calls.some((c) => c.command === "rotation_begin")).toBe(false);
  });
});

async function openGitHub(options: MockOptions) {
  const { ipc, calls } = recording({ start: "unlocked", credRotation: { stepMs: 0 }, ...options });
  const user = userEvent.setup();
  render(<App ipc={ipc} languages={["en"]} />);
  await user.click(await screen.findByRole("button", { name: /GitHub/ }));
  const article = await screen.findByRole("article", { name: "GitHub" });
  return { user, calls, article };
}

describe("item detail", () => {
  it("shows the open change, the not-confirmed entry and restores it", async () => {
    const { user, calls, article } = await openGitHub({ credRotation: { variant: "pending", stepMs: 0 } });
    expect(within(article).getByRole("heading", { name: /Password change started on/ })).toBeTruthy();
    expect(within(article).getByText(/The browser extension filled it into the site's form/)).toBeTruthy();
    expect(within(article).getByText("Not confirmed")).toBeTruthy();
    await user.click(within(article).getByRole("button", { name: /^Restore the password from/ }));
    await waitFor(() => {
      expect(calls.some((c) => c.command === "password_history_restore")).toBe(true);
    });
    // Confirming from the banner closes the attempt.
    await user.click(within(article).getByRole("button", { name: "It worked" }));
    await waitFor(() => {
      expect(within(article).queryByRole("heading", { name: /Password change started on/ })).toBeNull();
    });
  });

  it("sets the reminder interval", async () => {
    const { user, calls } = await openGitHub({ credRotation: { variant: "queue", stepMs: 0 } });
    const select = await screen.findByLabelText("Remind me to change this password");
    await user.selectOptions(select, "90");
    await waitFor(() => {
      expect(calls.some((c) => c.command === "rotation_set_policy" && JSON.stringify(c.request).includes('"everyDays":90'))).toBe(true);
    });
  });
});

describe("breach check card", () => {
  async function openOverview(options: MockOptions) {
    const { ipc, calls } = recording({ start: "unlocked", credRotation: { stepMs: 0 }, ...options });
    const user = userEvent.setup();
    render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: /^Security/ }));
    const card = await screen.findByRole("region", { name: "Breach check" });
    return { user, calls, card };
  }

  it("is off by default and asks first: the consent says what the service learns", async () => {
    const { user, calls, card } = await openOverview({ credRotation: { variant: "consent", stepMs: 0 } });
    expect(within(card).getByText(/Off\. Mosaic never contacts Have I Been Pwned/)).toBeTruthy();
    await user.click(within(card).getByRole("button", { name: "Check now" }));
    const dialog = await screen.findByRole("dialog", { name: "Check your passwords against known data breaches?" });
    expect(within(dialog).getByText(/first 5 characters of each password's SHA-1 hash/)).toBeTruthy();
    expect(within(dialog).getByText(/sees your IP address and those prefixes, never a password/)).toBeTruthy();
    expect(within(dialog).getByText(/Results stay on this device/)).toBeTruthy();
    // "Not now" is the focused default and sends nothing.
    await waitFor(() => {
      expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Not now" }));
    });
    await user.click(within(dialog).getByRole("button", { name: "Not now" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(calls.some((c) => c.command === "breach_scan_start")).toBe(false);
    expect(calls.some((c) => c.command === "update_settings")).toBe(false);
  });

  it("starts after the consent, shows a polite status and the result", async () => {
    const { user, calls, card } = await openOverview({ credRotation: { variant: "consent", stepMs: 0 } });
    await user.click(within(card).getByRole("button", { name: "Check now" }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Check now" }));
    await waitFor(() => {
      expect(calls.some((c) => c.command === "breach_scan_start")).toBe(true);
    });
    const saved = calls.find((c) => c.command === "update_settings")?.request as { settings: { breachCheck: string; breachConsentAt: number } };
    expect(saved.settings.breachCheck).toBe("manual");
    expect(saved.settings.breachConsentAt).toBeGreaterThan(0);
    // One polite status that changes at the start and the end, not per step.
    const status = within(card).getByRole("status");
    expect(status.getAttribute("aria-live")).toBe("polite");
    await waitFor(() => {
      expect(status.textContent).toMatch(/Checked \d+ passwords?\./);
    });
  });

  it("switches from off through the consent, and back to off without asking", async () => {
    const { user, calls, card } = await openOverview({ credRotation: { variant: "queue", stepMs: 0 } });
    // The queue variant is already on (manual): turning it off needs no dialog and withdraws the consent.
    const select = within(card).getByLabelText("Check against known data breaches");
    fireEvent.change(select, { target: { value: "off" } });
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
      expect(calls.some((c) => c.command === "update_settings" && JSON.stringify(c.request).includes('"breachConsentAt":null'))).toBe(true);
    });
    // Back on: the consent dialog opens again (the card shows "off" first).
    await within(card).findByText(/Off\. Mosaic never contacts/);
    fireEvent.change(within(card).getByLabelText("Check against known data breaches"), { target: { value: "weekly" } });
    await screen.findByRole("dialog", { name: "Check your passwords against known data breaches?" });
    await user.click(screen.getByRole("button", { name: "Not now" }));
    expect(calls.filter((c) => c.command === "breach_scan_start")).toHaveLength(0);
  });

  it("shows the offline error with the last results", async () => {
    const { user, card } = await openOverview({ credRotation: { variant: "breachOffline", stepMs: 0 } });
    expect(within(card).getByText(/Couldn't reach Have I Been Pwned/)).toBeTruthy();
    await user.click(within(card).getByRole("button", { name: "Check now" }));
    await waitFor(() => {
      expect(within(card).getAllByText(/Couldn't reach Have I Been Pwned/).length).toBeGreaterThan(0);
    });
  });
});
