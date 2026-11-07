import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import type { CommandName, RequestOf } from "../ipc/contract/index.ts";
import { createMockIpc } from "../ipc/mock.ts";
import type { MockSyncOptions } from "../ipc/mock/syncData.ts";

type User = ReturnType<typeof userEvent.setup>;

/** Renders the app on the unlocked mock and opens Settings; returns the Sync group. */
async function openSync(
  sync: MockSyncOptions = {},
  ipc: MosaicIpc = createMockIpc({ start: "unlocked", sync }),
  language = "en",
) {
  const user = userEvent.setup();
  render(<App ipc={ipc} languages={[language]} />);
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: language === "de" ? "Einstellungen" : "Settings" }));
  const heading = await screen.findByRole("heading", { name: language === "de" ? "Synchronisierung" : "Sync" });
  return { user, section: heading.closest("section") as HTMLElement };
}

/** The mock with a record of every call. */
function recording(sync: MockSyncOptions) {
  const inner = createMockIpc({ start: "unlocked", sync });
  const calls: { command: string; request: unknown }[] = [];
  const ipc: MosaicIpc = {
    subscribe: (listener) => inner.subscribe(listener),
    call: <K extends CommandName>(command: K, request: RequestOf<K>) => {
      calls.push({ command, request });
      return inner.call(command, request);
    },
  };
  return { ipc, calls };
}

function card(section: HTMLElement, name: string): HTMLElement {
  return within(section).getByRole("heading", { name }).closest(".card") as HTMLElement;
}

async function checkServer(user: User, section: HTMLElement, url = "https://sync.example.com") {
  const server = card(section, "Mosaic server");
  await user.type(within(server).getByLabelText("Server address"), url);
  await user.click(within(server).getByRole("button", { name: "Check server" }));
  return server;
}

describe("sync settings: not set up", () => {
  it("checks a server, then creates the sync account", async () => {
    const { user, section } = await openSync();
    expect(within(section).getByText(/Sync is off/)).toBeTruthy();
    const server = await checkServer(user, section);
    expect(await within(server).findByText("This server lets anyone create a sync account.")).toBeTruthy();
    expect(within(server).queryByLabelText("Invite code")).toBeNull();
    expect(within(server).getByLabelText<HTMLInputElement>("Name of this device").value).toMatch(
      /^Mosaic on (Windows|macOS|Linux)$/,
    );
    await user.type(within(server).getByLabelText("Master password"), "wrong");
    await user.click(within(server).getByRole("button", { name: "Create sync account" }));
    expect((await within(server).findAllByRole("alert")).some((a) => /master password or Secret Key didn't work/.test(a.textContent))).toBe(
      true,
    );
    await user.clear(within(server).getByLabelText("Master password"));
    await user.type(within(server).getByLabelText("Master password"), "mosaic");
    await user.click(within(server).getByRole("button", { name: "Create sync account" }));
    await waitFor(() => {
      expect(within(section).getByRole("button", { name: "Sync now" })).toBeTruthy();
    });
    expect(within(section).getByText("https://sync.example.com")).toBeTruthy();
    expect(within(section).getByText("alex@example.com")).toBeTruthy();
    expect(within(section).queryByRole("heading", { name: "Mosaic server" })).toBeNull();
  });

  it("shows the invite field only when the server needs an invite code", async () => {
    const { user, section } = await openSync({ server: "invite" });
    const server = await checkServer(user, section);
    expect(await within(server).findByText(/needs an invite code/)).toBeTruthy();
    await user.type(within(server).getByLabelText("Master password"), "mosaic");
    await user.type(within(server).getByLabelText("Invite code"), "nope");
    await user.click(within(server).getByRole("button", { name: "Create sync account" }));
    expect((await within(server).findAllByRole("alert")).some((a) => /invite code isn't valid/.test(a.textContent))).toBe(true);
    await user.clear(within(server).getByLabelText("Invite code"));
    await user.type(within(server).getByLabelText("Invite code"), "MOCK-INVITE");
    await user.click(within(server).getByRole("button", { name: "Create sync account" }));
    await within(section).findByRole("button", { name: "Sync now" });
  });

  it("offers only Sign in on a closed server", async () => {
    const { user, section } = await openSync({ server: "closed" });
    const server = await checkServer(user, section);
    expect(await within(server).findByText(/doesn't accept new sync accounts/)).toBeTruthy();
    expect(within(server).queryByRole("button", { name: "Create sync account" })).toBeNull();
    expect(within(server).getByRole("button", { name: "Sign in" })).toBeTruthy();
  });

  it("explains an address that is not a server", async () => {
    const { user, section } = await openSync();
    await checkServer(user, section, "http://sync.example.com");
    expect((await within(section).findAllByRole("alert")).some((a) => /only connects to servers over https/.test(a.textContent))).toBe(
      true,
    );
  });

  it("asks for the 2FA code when signing in to an account that has one", async () => {
    const { user, section } = await openSync({ variant: "hosted" });
    await user.click(within(section).getByRole("button", { name: "Disconnect…" }));
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Disconnect" }));
    const server = await checkServer(user, section);
    await user.type(await within(server).findByLabelText("Master password"), "mosaic");
    await user.click(within(server).getByRole("button", { name: "Sign in" }));
    await within(server).findByRole("heading", { name: "Enter your code" });
    await user.type(within(server).getByLabelText("6-digit code"), "000000");
    await user.click(within(server).getByRole("button", { name: "Verify" }));
    expect((await within(server).findAllByRole("alert")).some((a) => /code isn't right/.test(a.textContent))).toBe(true);
    await user.clear(within(server).getByLabelText("6-digit code"));
    await user.type(within(server).getByLabelText("6-digit code"), "123456");
    await user.click(within(server).getByRole("button", { name: "Verify" }));
    await within(section).findByRole("button", { name: "Sync now" });
    expect(within(section).getByText("Two-step verification is on.")).toBeTruthy();
  });

  it("connects an empty folder", async () => {
    const { user, section } = await openSync({ folder: "empty" });
    const folder = card(section, "Your own cloud folder");
    expect(within(folder).queryByRole("button", { name: "Connect folder" })).toBeNull();
    await user.click(within(folder).getByRole("button", { name: "Choose folder…" }));
    expect(await within(folder).findByText("D:\\Sync")).toBeTruthy();
    expect(within(folder).getByText("The folder is empty. Mosaic can use it.")).toBeTruthy();
    await user.click(within(folder).getByRole("button", { name: "Connect folder" }));
    await within(section).findByRole("button", { name: "Sync now" });
    expect(within(section).getByText(/To lock a device out, change your master password and Secret Key/)).toBeTruthy();
    // A folder has no sync account: no two-step verification, no Remove.
    expect(within(section).queryByRole("heading", { name: "Two-step verification" })).toBeNull();
    expect(within(section).queryByRole("button", { name: /^Remove / })).toBeNull();
  });

  it("refuses a folder with other files", async () => {
    const { user, section } = await openSync({ folder: "other" });
    const folder = card(section, "Your own cloud folder");
    await user.click(within(folder).getByRole("button", { name: "Choose folder…" }));
    expect(await within(folder).findByText(/holds other files\. Mosaic writes nothing there/)).toBeTruthy();
    expect(within(folder).queryByRole("button", { name: "Connect folder" })).toBeNull();
  });

  it("keeps the page as it is when the folder dialog is cancelled", async () => {
    const { user, section } = await openSync({ folder: "cancel" });
    const folder = card(section, "Your own cloud folder");
    await user.click(within(folder).getByRole("button", { name: "Choose folder…" }));
    expect(within(folder).queryByText("Folder:")).toBeNull();
    expect(within(folder).getByRole("button", { name: "Choose folder…" })).toBeTruthy();
  });

  it("says what each location can see", async () => {
    const { section } = await openSync();
    expect(within(section).getByText(/The server sees only encrypted data/)).toBeTruthy();
    expect(within(section).getByText(/Your cloud provider and anyone with access to the folder see only encrypted data/)).toBeTruthy();
  });
});

describe("sync settings: connected", () => {
  it("shows the status, syncs now and lists the devices", async () => {
    const { user, section } = await openSync({ variant: "hosted" });
    expect(await within(section).findByText(/Synced · Last synced 5 minutes ago/)).toBeTruthy();
    expect(within(section).getByText("Mosaic on macOS")).toBeTruthy();
    expect(within(section).getByText("Unnamed device")).toBeTruthy();
    expect(within(section).getAllByText("This device").length).toBeGreaterThan(0);
    expect(within(section).getByText("Two-step verification is on.")).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: "Sync now" }));
    await within(section).findByText(/Last synced (now|0 seconds ago|in 0 seconds)/);
  });

  it.each([
    ["syncing", /Syncing…/],
    ["offline", /Offline/],
    ["error", /Sync failed/],
  ] as const)("shows the %s state in a status region", async (variant, text) => {
    const { section } = await openSync({ variant });
    const status = await within(section).findByText(text, { selector: "p[role=status]" });
    expect(status.getAttribute("aria-live")).toBe("polite");
  });

  it("shows an error code of the last run with what to do", async () => {
    const { section } = await openSync({ variant: "offline" });
    await waitFor(() => {
      expect(within(section).getAllByRole("alert").some((a) => /couldn't reach the server/.test(a.textContent))).toBe(true);
    });
  });

  it("shows the failure of Sync now", async () => {
    const { user, section } = await openSync({ variant: "hosted", fail: "syncQuotaExceeded" });
    await user.click(await within(section).findByRole("button", { name: "Sync now" }));
    await waitFor(() => {
      expect(within(section).getAllByRole("alert").some((a) => /no room left/.test(a.textContent))).toBe(true);
    });
  });

  it("renames this device", async () => {
    const { calls, ipc } = recording({ variant: "hosted" });
    const { user, section } = await openSync({}, ipc);
    await user.click(await within(section).findByRole("button", { name: "Rename Mosaic on Windows" }));
    const input = within(section).getByLabelText("New name for this device");
    await user.clear(input);
    await user.type(input, "Work laptop");
    await user.click(within(section).getByRole("button", { name: "Save" }));
    await waitFor(() => {
      expect(within(section).getByRole("button", { name: "Rename Work laptop" })).toBeTruthy();
    });
    expect(calls.some((call) => call.command === "sync_device_rename")).toBe(true);
    expect(within(section).getAllByText("Work laptop").length).toBeGreaterThan(0);
  });

  it("removes another device after the master password and a code", async () => {
    const { user, section } = await openSync({ variant: "hosted" });
    await user.click(await within(section).findByRole("button", { name: "Remove Mosaic on macOS" }));
    const dialog = within(screen.getByRole("dialog"));
    expect(dialog.getByText(/signs it out of sync\. It doesn't take away the copy of your vaults/)).toBeTruthy();
    await user.type(dialog.getByLabelText("Master password"), "mosaic");
    await user.click(dialog.getByRole("button", { name: "Remove device" }));
    expect((await dialog.findAllByRole("alert")).some((a) => /needs a code/.test(a.textContent))).toBe(true);
    await user.type(dialog.getByLabelText("Master password"), "mosaic");
    await user.type(dialog.getByLabelText("6-digit code"), "123456");
    await user.click(dialog.getByRole("button", { name: "Remove device" }));
    await waitFor(() => {
      expect(within(section).queryByText("Mosaic on macOS")).toBeNull();
    });
    expect(within(section).queryByRole("button", { name: "Remove Mosaic on Windows" })).toBeNull();
  });

  it("asks no code to remove a device when the account has no second factor", async () => {
    const { user, section } = await openSync({ variant: "hostedNoTotp" });
    await user.click(await within(section).findByRole("button", { name: "Remove Mosaic on macOS" }));
    expect(within(screen.getByRole("dialog")).queryByLabelText("6-digit code")).toBeNull();
  });

  it("turns two-step verification on, then off", async () => {
    const { user, section } = await openSync({ variant: "hostedNoTotp" });
    expect(await within(section).findByText("Two-step verification is off.")).toBeTruthy();
    expect(within(section).getByText(/It protects the server account, not your vaults/)).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: "Turn on…" }));
    let dialog = within(screen.getByRole("dialog"));
    await user.type(dialog.getByLabelText("Master password"), "mosaic");
    await user.click(dialog.getByRole("button", { name: "Continue" }));
    await waitFor(() => {
      expect(within(screen.getByRole("dialog")).getByLabelText<HTMLInputElement>("Setup key").value).toBe(
        "JBSWY3DPEHPK3PXP",
      );
    });
    dialog = within(screen.getByRole("dialog"));
    await user.type(dialog.getByLabelText("6-digit code"), "123456");
    await user.click(dialog.getByRole("button", { name: "Turn on" }));
    await within(section).findByText("Two-step verification is on.");
    expect(screen.queryByRole("dialog")).toBeNull();

    await user.click(within(section).getByRole("button", { name: "Turn off…" }));
    dialog = within(screen.getByRole("dialog"));
    await user.type(dialog.getByLabelText("Master password"), "mosaic");
    await user.type(dialog.getByLabelText("6-digit code"), "123456");
    await user.click(dialog.getByRole("button", { name: "Turn off" }));
    await within(section).findByText("Two-step verification is off.");
  });

  it("shows a wrong two-step code in the setup dialog", async () => {
    const { user, section } = await openSync({ variant: "hostedNoTotp" });
    await user.click(await within(section).findByRole("button", { name: "Turn on…" }));
    await user.type(within(screen.getByRole("dialog")).getByLabelText("Master password"), "mosaic");
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Continue" }));
    const code = await within(screen.getByRole("dialog")).findByLabelText("6-digit code");
    await user.type(code, "000000");
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Turn on" }));
    expect(
      (await within(screen.getByRole("dialog")).findAllByRole("alert")).some((a) => /code isn't right/.test(a.textContent)),
    ).toBe(true);
  });

  it("disconnects only after a confirmation", async () => {
    const { user, section } = await openSync({ variant: "hosted" });
    await user.click(await within(section).findByRole("button", { name: "Disconnect…" }));
    expect(within(screen.getByRole("dialog")).getByText(/Your vaults stay on this device/)).toBeTruthy();
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Cancel" }));
    expect(within(section).getByRole("button", { name: "Sync now" })).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: "Disconnect…" }));
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Disconnect" }));
    await within(section).findByRole("heading", { name: "Mosaic server" });
    expect(within(section).getByRole("heading", { name: "Your own cloud folder" })).toBeTruthy();
  });

  it("signs in again when the server signed this device out", async () => {
    const { user, section } = await openSync({ variant: "signedOut" });
    expect(await within(section).findByRole("heading", { name: "This device is signed out" })).toBeTruthy();
    expect(within(section).queryByRole("button", { name: "Sync now" })).toBeNull();
    expect(within(section).getByText(/^Signed out · /, { selector: "p[role=status]" })).toBeTruthy();
    await user.type(within(section).getByLabelText("Master password"), "mosaic");
    await user.click(within(section).getByRole("button", { name: "Sign in again" }));
    await within(section).findByRole("heading", { name: "Enter your code" });
    await user.type(within(section).getByLabelText("6-digit code"), "123456");
    await user.click(within(section).getByRole("button", { name: "Verify" }));
    await within(section).findByRole("button", { name: "Sync now" });
  });

  it("shows the sidebar dot as an error when signed out", async () => {
    await openSync({ variant: "signedOut" });
    await waitFor(() => {
      expect(document.querySelector(".sidebar__sync-text")?.textContent).toBe("Signed out");
    });
    expect(document.querySelector(".sync-dot--error")).not.toBeNull();
  });

  it("is in German", async () => {
    const { section } = await openSync({ variant: "hosted" }, undefined, "de");
    expect(await within(section).findByText(/Zuletzt synchronisiert/)).toBeTruthy();
    expect(within(section).getByRole("button", { name: "Jetzt synchronisieren" })).toBeTruthy();
  });
});

describe("sync settings: warnings", () => {
  const messages: readonly [string, RegExp][] = [
    ["rollbackDetected", /^Your sync data looks older than this device\. Check the server or folder, then sync again\.$/],
    ["identityDuplicated", /^Another copy of Mosaic is writing under this device's identity/],
    ["serverInconsistent", /^The server showed this device different data than another device\. Check the server, then confirm\.$/],
    ["serverRollback", /^The server's copy of .*Work.* is older than this device's\. Check the server, then confirm\.$/],
    [
      "noCommonView",
      /^This device and .*Mosaic on macOS.* haven't seen the same server data for two days\. The server may be showing them different versions\.$/,
    ],
    ["freezeSuspected", /^Changes from .*Mosaic on macOS.* have not arrived for over a day/],
    ["missingFiles", /^Files from .*Mosaic on Linux.* are missing from the folder/],
    ["undecryptable", /^3 received items couldn't be read on this device/],
    ["hardwareVaultsLocal", /^Hardware-protected vaults stay on this device\. 1 vault isn't synced\.$/],
    ["attachmentsNotRepaired", /^2 files on the server or folder couldn't be repaired/],
  ];

  it("renders the message of every warning kind", async () => {
    const { section } = await openSync({ variant: "allWarnings" });
    const group = await within(section).findByRole("group", { name: "Sync warnings" });
    expect(messages).toHaveLength(10);
    for (const [kind, pattern] of messages) {
      expect(within(group).getByText(pattern), kind).toBeTruthy();
    }
  });

  it("renders every warning in German too", async () => {
    const { section } = await openSync({ variant: "allWarnings" }, undefined, "de");
    const group = await within(section).findByRole("group", { name: "Sync-Warnungen" });
    expect(within(group).getAllByText(/\S/).length).toBeGreaterThanOrEqual(10);
    expect(within(group).getByText(/^Zwischen diesem Gerät und .*Mosaic on macOS.*: Seit zwei Tagen/)).toBeTruthy();
    expect(within(group).getByText(/^Hardwaregeschützte Tresore bleiben auf diesem Gerät/)).toBeTruthy();
  });

  it("offers Acknowledge… for exactly the five halt and finding kinds", async () => {
    const { section } = await openSync({ variant: "allWarnings" });
    const group = await within(section).findByRole("group", { name: "Sync warnings" });
    expect(within(group).getAllByRole("button", { name: "Acknowledge…" })).toHaveLength(5);
  });

  it("acknowledges after a confirmation that explains the risk", async () => {
    const { calls, ipc } = recording({ variant: "rollback" });
    const { user, section } = await openSync({}, ipc);
    await user.click(await within(section).findByRole("button", { name: "Acknowledge…" }));
    const dialog = within(screen.getByRole("dialog"));
    expect(dialog.getByText(/Only go on after you have checked the server or folder yourself/)).toBeTruthy();
    expect(dialog.getByText("Do this on each of your devices.")).toBeTruthy();
    // Cancel changes nothing.
    await user.click(dialog.getByRole("button", { name: "Cancel" }));
    expect(calls.some((call) => call.command === "sync_acknowledge")).toBe(false);
    expect(within(section).getByText(/looks older than this device/)).toBeTruthy();

    await user.click(within(section).getByRole("button", { name: "Acknowledge…" }));
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Acknowledge and sync" }));
    await waitFor(() => {
      expect(within(section).queryByText(/looks older than this device/)).toBeNull();
    });
    expect(calls.filter((call) => call.command === "sync_acknowledge")).toHaveLength(1);
    expect(within(section).getByText(/Synced/, { selector: "p[role=status]" })).toBeTruthy();
  });
});
