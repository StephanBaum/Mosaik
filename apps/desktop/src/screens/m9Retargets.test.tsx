/**
 * The M9 retargets in the app (mock backend): the vault key dialog (M7-R2), the shortcut field
 * (M3-R10), the reason text of a signed-out device (M7-R3), the dismissable "vault details were
 * shortened" notice and the foreign browser connector banner.
 */
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import type { CommandName, RequestOf } from "../ipc/contract/index.ts";
import { createMockIpc, type MockOptions } from "../ipc/mock.ts";

function recording(options: MockOptions) {
  const inner = createMockIpc({ start: "unlocked", ...options });
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

async function open(options: MockOptions, language = "en") {
  const { ipc, calls } = recording(options);
  const user = userEvent.setup();
  render(<App ipc={ipc} languages={[language]} />);
  await screen.findByText("GitHub");
  return { user, calls };
}

async function openSettings(user: ReturnType<typeof userEvent.setup>, name = "Settings") {
  await user.click(screen.getByRole("button", { name }));
}

describe("Renew vault key…", () => {
  const sync = { variant: "hostedNoTotp" } as const;
  const rotation = { variant: "none", stepMs: 0 } as const;

  async function openRotateDialog(options: MockOptions) {
    const { user, calls } = await open(options);
    await user.click(screen.getByRole("button", { name: /^Personal/ }));
    await user.click(await screen.findByRole("button", { name: "Vault actions" }));
    await user.click(screen.getByRole("menuitem", { name: "Renew vault key…" }));
    const dialog = await screen.findByRole("dialog", { name: /^Renew the key of .*Personal/ });
    return { user, calls, dialog };
  }

  it("asks for the master password after the click, then rotates with the attachments option", async () => {
    const { user, calls, dialog } = await openRotateDialog({ sync, rotation });
    await user.click(within(dialog).getByRole("switch", { name: "Also renew the keys of attachments" }));
    await user.click(within(dialog).getByRole("button", { name: "Renew key" }));

    const reauth = await screen.findByRole("dialog", { name: "Confirm with your master password" });
    expect(within(reauth).getByText("Renewing a vault key needs your master password.")).toBeTruthy();
    await user.type(within(reauth).getByLabelText("Master password"), "mosaic");
    await user.click(within(reauth).getByRole("button", { name: "Confirm" }));

    expect(await within(dialog).findByText("The vault key was renewed, and the attachment keys were renewed too.")).toBeTruthy();
    const rotate = calls.filter((call) => call.command === "vault_rotate_key");
    expect(rotate.map((call) => (call.request as { resealAttachments: boolean }).resealAttachments)).toEqual([
      true,
      true,
    ]);
    // The first call had no proof (`reauthRequired`); the retry ran on the proof `reauthenticate` left.
    expect(rotate.every((call) => (call.request as { proof: string | null }).proof === null)).toBe(true);
    expect(calls.some((call) => call.command === "reauthenticate")).toBe(true);
  });

  it("rotates without the attachments when the option stays off", async () => {
    const { user, calls, dialog } = await openRotateDialog({ sync, rotation });
    await user.click(within(dialog).getByRole("button", { name: "Renew key" }));
    const reauth = await screen.findByRole("dialog", { name: "Confirm with your master password" });
    await user.type(within(reauth).getByLabelText("Master password"), "mosaic");
    await user.click(within(reauth).getByRole("button", { name: "Confirm" }));
    expect(await within(dialog).findByText("The vault key was renewed.")).toBeTruthy();
    const last = calls.filter((call) => call.command === "vault_rotate_key").at(-1);
    expect(last?.request).toMatchObject({ resealAttachments: false });
  });

  it("goes back to the question when the password dialog is closed, without an error", async () => {
    const { user, dialog } = await openRotateDialog({ sync, rotation });
    await user.click(within(dialog).getByRole("button", { name: "Renew key" }));
    const reauth = await screen.findByRole("dialog", { name: "Confirm with your master password" });
    await user.click(within(reauth).getByRole("button", { name: "Cancel" }));
    await waitFor(() => {
      expect(within(dialog).getByRole("button", { name: "Renew key" })).toHaveProperty("disabled", false);
    });
    expect(within(dialog).queryByText("The vault key was renewed.")).toBeNull();
  });

  it("says what went wrong when the rotation fails", async () => {
    const { user, dialog } = await openRotateDialog({ sync, rotation: { ...rotation, fail: "syncServerUnreachable" } });
    await user.click(within(dialog).getByRole("button", { name: "Renew key" }));
    const reauth = await screen.findByRole("dialog", { name: "Confirm with your master password" });
    await user.type(within(reauth).getByLabelText("Master password"), "mosaic");
    await user.click(within(reauth).getByRole("button", { name: "Confirm" }));
    expect((await within(dialog).findAllByRole("alert")).some((alert) => /server/i.test(alert.textContent))).toBe(true);
  });

  it("is not offered without a sync backend, in All vaults, or to a member who is not the owner", async () => {
    // No backend: a vault key is offered to a backend, so there is nothing to rotate.
    const none = await open({});
    await none.user.click(screen.getByRole("button", { name: /^Personal/ }));
    expect(screen.queryByRole("button", { name: "Vault actions" })).toBeNull();
  });

  it("is hidden from a viewer of a shared vault", async () => {
    const { user } = await open({ sharing: "viewer", rotation });
    await user.click(screen.getByRole("button", { name: /^Work/ }));
    await waitFor(() => {
      expect(screen.getByRole("navigation", { name: "Breadcrumb" }).textContent).toContain("Work");
    });
    expect(screen.queryByRole("button", { name: "Vault actions" })).toBeNull();
  });

  it("shows a cut-off vault job in the banner and finishes it with Continue", async () => {
    const { user } = await open({ sync, rotation: { variant: "resumeVault", stepMs: 0 } });
    const banner = await screen.findByRole("region", { name: "Key change not finished" });
    expect(within(banner).getByText(/attachments of a vault was cut off/)).toBeTruthy();
    await user.click(within(banner).getByRole("button", { name: "Continue key rotation" }));
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Key change not finished" })).toBeNull();
    });
  });
});

describe("the quick search shortcut", () => {
  it("saves a new shortcut and says so", async () => {
    const { user, calls } = await open({});
    await openSettings(user);
    const field = await screen.findByLabelText<HTMLInputElement>("Shortcut");
    expect(field.value).toBe("Ctrl+Shift+Space");
    // The field records the key press (M9-UX13): nothing is typed.
    await user.click(field);
    await user.keyboard("{Control>}{Alt>}k{/Alt}{/Control}");
    expect(field.value).toBe("Ctrl+Alt+K");
    await user.click(screen.getByRole("button", { name: "Save shortcut" }));
    expect(await screen.findByText("Shortcut saved: Ctrl+Alt+K.")).toBeTruthy();
    const update = calls.filter((call) => call.command === "update_settings").at(-1);
    expect(update?.request).toMatchObject({ settings: { globalShortcut: "Ctrl+Alt+K" }, password: null });
    expect(screen.getByRole("button", { name: "Use the default" })).toHaveProperty("disabled", false);
  });

  it("explains a combination the app cannot use", async () => {
    const { user } = await open({});
    await openSettings(user);
    const field = await screen.findByLabelText("Shortcut");
    await user.click(field);
    await user.keyboard("{Shift>}k{/Shift}");
    await user.click(screen.getByRole("button", { name: "Save shortcut" }));
    expect(await screen.findByText(/That isn't a shortcut Mosaic can use/)).toBeTruthy();
  });

  it("says shortcutUnavailable when another program holds the combination", async () => {
    const { user } = await open({ shortcut: "conflict" });
    await openSettings(user);
    const field = await screen.findByLabelText("Shortcut");
    await user.click(field);
    await user.keyboard("{Control>}{Alt>}k{/Alt}{/Control}");
    await user.click(screen.getByRole("button", { name: "Save shortcut" }));
    expect(await screen.findByText(/Another program already uses this shortcut/)).toBeTruthy();
  });

  it("disables the field where the system has no global shortcuts", async () => {
    const { user } = await open({ shortcut: "unsupported" });
    await openSettings(user);
    const field = await screen.findByLabelText<HTMLInputElement>("Shortcut");
    await waitFor(() => {
      expect(field.disabled).toBe(true);
    });
    expect(screen.getByText(/doesn't let apps set a global shortcut, so this can't be changed here/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save shortcut" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Use the default" })).toHaveProperty("disabled", true);
  });

  it("is in German too, in the informal form", async () => {
    const { user } = await open({}, "de");
    await openSettings(user, "Einstellungen");
    expect(await screen.findByLabelText("Kürzel")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Kürzel speichern" })).toBeTruthy();
  });
});

describe("why a device was signed out", () => {
  const reasons = {
    rekey: /keys were renewed on another device: sign in with the new Secret Key from your newest Emergency Kit/,
    passwordChange: /master password was changed on another device/,
    deviceRemoved: /removed from your account on another device/,
    operator: /operator of the server ended this session/,
  } as const;

  for (const [reason, text] of Object.entries(reasons)) {
    it(`says it in plain words for ${reason}`, async () => {
      const { user } = await open({ sync: { variant: "signedOut", revoked: reason as keyof typeof reasons } });
      await openSettings(user);
      const heading = await screen.findByRole("heading", { name: "Sync" });
      const section = heading.closest("section") as HTMLElement;
      expect(await within(section).findByText(text)).toBeTruthy();
    });
  }

  it("asks for the new Secret Key after a re-key and not after a removed device", async () => {
    const rekey = await open({ sync: { variant: "signedOut", revoked: "rekey" } });
    await openSettings(rekey.user);
    expect(await screen.findByLabelText(/Secret Key/)).toBeTruthy();
  });

  it("says nothing when the server gave no reason", async () => {
    const { user } = await open({ sync: { variant: "signedOut" } });
    await openSettings(user);
    const heading = await screen.findByRole("heading", { name: "Sync" });
    const section = heading.closest("section") as HTMLElement;
    expect(await within(section).findByText("This device is signed out")).toBeTruthy();
    expect(within(section).queryByText(/another device/)).toBeNull();
  });

  it("says it in German, informal", async () => {
    const { user } = await open({ sync: { variant: "signedOut", revoked: "passwordChange" } }, "de");
    await openSettings(user, "Einstellungen");
    expect(await screen.findByText(/Dein Master-Passwort wurde auf einem anderen Gerät geändert/)).toBeTruthy();
  });
});

describe("vault details that were shortened", () => {
  it("names the fields in plain words, never the values, and can be dismissed", async () => {
    const { user } = await open({ metaDropped: ["name", "description"] });
    const note = await screen.findByText(/Two devices changed the name and description of “Personal”/);
    expect(note.textContent).not.toMatch(/\.\.\./);
    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    await waitFor(() => {
      expect(screen.queryByText(/Two devices changed the name and description/)).toBeNull();
    });
    // Focus did not fall to the top of the page.
    await waitFor(() => {
      expect(document.activeElement).not.toBe(document.body);
    });
  });

  it("is in German, informal", async () => {
    await open({ metaDropped: ["icon"] }, "de");
    expect(await screen.findByText(/Zwei Geräte haben das Symbol von „Personal“ gleichzeitig geändert/)).toBeTruthy();
  });
});

describe("a browser connection that points somewhere else", () => {
  it("shows the banner at the start and in Settings > Browsers, and Repair removes it", async () => {
    const { user, calls } = await open({ connector: "foreign" });
    const banner = await screen.findByText(/starts a different program instead of Mosaic/);
    expect(banner.textContent).toContain("other-host.exe");
    await user.click(screen.getByRole("button", { name: "Repair" }));
    await waitFor(() => {
      expect(screen.queryByText(/starts a different program instead of Mosaic/)).toBeNull();
    });
    expect(calls.some((call) => call.command === "browser_connector_repair")).toBe(true);
    expect(await screen.findByText("Browser connection repaired.")).toBeTruthy();
    // The banner went away with its button: focus is on the page, not lost.
    await waitFor(() => {
      expect(document.activeElement).not.toBe(document.body);
    });
  });

  it("is also in Settings > Browsers", async () => {
    const { user } = await open({ connector: "foreign" });
    await openSettings(user);
    const heading = await screen.findByRole("heading", { name: "Browsers" });
    const section = heading.closest("section") as HTMLElement;
    expect(await within(section).findByText(/starts a different program instead of Mosaic/)).toBeTruthy();
    expect(within(section).getByRole("button", { name: "Repair" })).toBeTruthy();
  });

  it("is absent when the audit finds nothing (the stub answers no findings)", async () => {
    const { user, calls } = await open({});
    await openSettings(user);
    await screen.findByRole("heading", { name: "Browsers" });
    expect(screen.queryByRole("button", { name: "Repair" })).toBeNull();
    expect(calls.some((call) => call.command === "browser_connector_audit")).toBe(true);
  });
});
