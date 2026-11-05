import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SecretsProvider } from "../../app/SecretsContext.tsx";
import { ToastProvider } from "../../components/ToastProvider.tsx";
import { I18nProvider } from "../../i18n/I18nProvider.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { CommandName, IpcError, ItemDetail } from "../../ipc/contract/index.ts";
import { IpcProvider } from "../../ipc/IpcProvider.tsx";
import { createMockIpc } from "../../ipc/mock.ts";
import { ItemEditor } from "./ItemEditor.tsx";

function renderEditor() {
  const onCancel = vi.fn();
  render(
    <I18nProvider locale="en">
      <IpcProvider ipc={createMockIpc({ start: "unlocked" })}>
        <ToastProvider>
          <ItemEditor
            vaults={[{ vaultId: "v1", name: "Personal", itemCount: 0, available: true, protection: "accountKey", openUntil: null }]}
            vaultId="v1"
            editing={null}
            onSaved={vi.fn()}
            onCancel={onCancel}
          />
        </ToastProvider>
      </IpcProvider>
    </I18nProvider>,
  );
  return { onCancel };
}

describe("item editor", () => {
  it("cancels on Esc when nothing changed", async () => {
    const user = userEvent.setup();
    const { onCancel } = renderEditor();
    await user.click(screen.getByLabelText("Title"));
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("asks before it drops edits", async () => {
    const user = userEvent.setup();
    const { onCancel } = renderEditor();
    await user.type(screen.getByLabelText("Title"), "Draft");
    await user.keyboard("{Escape}");
    expect(onCancel).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog", { name: "Discard your changes?" });
    expect(dialog).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Discard" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("puts the cursor in the title and leaves on Esc from the page body", async () => {
    const user = userEvent.setup();
    const { onCancel } = renderEditor();
    expect(document.activeElement).toBe(screen.getByLabelText("Title"));
    // After a click on empty space or a closed menu, focus is on the body: Esc still leaves.
    (document.activeElement as HTMLElement).blur();
    expect(document.activeElement).toBe(document.body);
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("asks before it drops edits when Esc comes from the page body", async () => {
    const user = userEvent.setup();
    const { onCancel } = renderEditor();
    await user.type(screen.getByLabelText("Title"), "Draft");
    (document.activeElement as HTMLElement).blur();
    await user.keyboard("{Escape}");
    expect(onCancel).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog", { name: "Discard your changes?" });
    // The dialog starts on "Keep editing", and its own Esc keeps the editor open.
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Keep editing" }));
    // Esc in a dialog is its own cancel event (jsdom does not send it for a key press).
    fireEvent(dialog, new Event("cancel", { cancelable: true }));
    expect(dialog.isConnected).toBe(false);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("keeps the notes in a multi-line field", () => {
    renderEditor();
    const notes = screen.getByLabelText("Notes");
    expect(notes.tagName).toBe("TEXTAREA");
  });
});

interface Sent {
  command: CommandName;
  request: unknown;
}

/** The mock with a hardware vault open, recording what the editor sends. */
async function renderWithHardwareVault(failTotp?: IpcError) {
  const inner = createMockIpc({ start: "unlocked", quick: ["hardwareKey"] });
  const listed = await inner.call("list_vaults", null);
  const vaults = listed.ok ? listed.value : [];
  const hardware = vaults.find((vault) => vault.protection === "hardwareKey");
  await inner.call("open_hardware_vault", { vaultId: hardware?.vaultId ?? "", pin: "1234" });
  const opened = await inner.call("list_vaults", null);
  const sent: Sent[] = [];
  const ipc: MosaicIpc = {
    subscribe: (listener) => inner.subscribe(listener),
    call: (command, request) => {
      sent.push({ command, request });
      const isTotp = command === "add_item" && (request as { kind: string }).kind === "totp";
      if (failTotp !== undefined && isTotp) {
        return Promise.resolve({ ok: false, error: failTotp });
      }
      return inner.call(command, request);
    },
  };
  const all = opened.ok ? opened.value : [];
  render(
    <I18nProvider locale="en">
      <IpcProvider ipc={ipc}>
        <ToastProvider>
          <SecretsProvider>
            <ItemEditor
              vaults={all}
              vaultId={all[0]?.vaultId ?? null}
              editing={null}
              onSaved={vi.fn()}
              onCancel={vi.fn()}
            />
          </SecretsProvider>
        </ToastProvider>
      </IpcProvider>
    </I18nProvider>,
  );
  return { sent, hardwareId: hardware?.vaultId ?? "", personalId: all[0]?.vaultId ?? "" };
}

async function scanAndUse(user: ReturnType<typeof userEvent.setup>, choice?: RegExp) {
  await user.type(screen.getByLabelText("Title"), "Example");
  await user.click(await screen.findByRole("button", { name: "Scan screen" }));
  await screen.findByRole("heading", { name: "Is this the right code?" });
  if (choice !== undefined) {
    await user.click(screen.getByRole("radio", { name: choice }));
  }
  await user.click(screen.getByRole("button", { name: "Use this code" }));
  await screen.findByText("Saved together with this item.");
}

const adds = (sent: Sent[]) => sent.filter((each) => each.command === "add_item");

describe("item editor, one-time code", () => {
  it("saves a scanned code as setPending with the confirmed labels", async () => {
    const user = userEvent.setup();
    const { sent } = await renderWithHardwareVault();
    await scanAndUse(user);
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => {
      expect(adds(sent)).toHaveLength(1);
    });
    const draft = (adds(sent)[0]?.request as { draft: { otp: unknown } }).draft;
    expect(draft.otp).toEqual({
      op: "setPending",
      pendingId: "otp-1",
      issuer: "GitHub",
      accountName: "alex@example.com",
    });
  });

  it("stores a code meant for a hardware vault there first, then saves the login without it", async () => {
    const user = userEvent.setup();
    const { sent, hardwareId, personalId } = await renderWithHardwareVault();
    await scanAndUse(user, /needs your security key/);
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => {
      expect(adds(sent)).toHaveLength(2);
    });
    const [first, second] = adds(sent).map((each) => each.request as {
      vaultId: string;
      kind: string;
      draft: { otp: unknown };
    });
    expect(first?.vaultId).toBe(hardwareId);
    expect(first?.kind).toBe("totp");
    expect(first?.draft.otp).toMatchObject({ op: "setPending", pendingId: "otp-1" });
    expect(second?.vaultId).toBe(personalId);
    expect(second?.kind).toBe("login");
    expect(second?.draft.otp).toBeNull();
  });

  it("does not save the login when the code could not be stored", async () => {
    const user = userEvent.setup();
    const { sent } = await renderWithHardwareVault({ code: "vaultUnavailable" });
    await scanAndUse(user, /needs your security key/);
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => {
      expect(screen.getByRole("alert").textContent).not.toBe("");
    });
    expect(adds(sent)).toHaveLength(1);
  });

  it("forgets a pending code when the editor is cancelled", async () => {
    const user = userEvent.setup();
    const { sent } = await renderWithHardwareVault();
    await scanAndUse(user);
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(sent.some((each) => each.command === "otp_pending_discard")).toBe(true);
  });

  it("blocks saving a one-time code item that has no code", async () => {
    const user = userEvent.setup();
    const { sent } = await renderWithHardwareVault();
    await user.selectOptions(screen.getByLabelText("Type"), "totp");
    await user.type(screen.getByLabelText("Title"), "Bank");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Add a one-time code to save this item.")).toBeTruthy();
    expect(adds(sent)).toHaveLength(0);
  });
});

const VAULTS = [
  { vaultId: "v1", name: "Personal", itemCount: 0, available: true, protection: "accountKey", openUntil: null },
] as const;

function stored(over: Partial<ItemDetail> = {}): ItemDetail {
  return {
    summary: {
      vaultId: "v1",
      itemId: "i1",
      kind: "login",
      title: "GitHub",
      subtitle: null,
      host: null,
      favorite: false,
      hasOtp: false,
      tags: [],
      modifiedAt: null,
      trashedAt: null,
      needsReview: false,
    },
    createdAt: null,
    text: [{ field: "username", value: "alex" }],
    secrets: ["password"],
    urls: [],
    customFields: [{ elementId: "c1", label: "PIN", kind: "hidden", value: null }],
    attachments: [],
    otp: null,
    historyCount: 0,
    review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
    sshKey: null,
    ...over,
  };
}

/** The editor on an existing item, recording what it sends. */
function renderEditing(editing: ItemDetail) {
  const inner = createMockIpc({ start: "unlocked" });
  const sent: Sent[] = [];
  const ipc: MosaicIpc = {
    subscribe: (listener) => inner.subscribe(listener),
    call: (command, request) => {
      sent.push({ command, request });
      return inner.call(command, request);
    },
  };
  render(
    <I18nProvider locale="en">
      <IpcProvider ipc={ipc}>
        <ToastProvider>
          <ItemEditor
            vaults={VAULTS}
            vaultId="v1"
            editing={editing}
            onSaved={vi.fn()}
            onCancel={vi.fn()}
          />
        </ToastProvider>
      </IpcProvider>
    </I18nProvider>,
  );
  return sent;
}

describe("item editor, kinds and custom fields", () => {
  it("shows the fields of the chosen kind", async () => {
    const user = userEvent.setup();
    renderEditor();
    expect(screen.getByLabelText("User name")).toBeTruthy();
    await user.selectOptions(screen.getByLabelText("Type"), "creditCard");
    expect(screen.getByLabelText("Card number")).toBeTruthy();
    expect(screen.getByLabelText("Cardholder")).toBeTruthy();
    expect(screen.queryByLabelText("User name")).toBeNull();
    expect(screen.queryByLabelText("Website 1")).toBeNull();
  });

  it("hints at a card number that fails the check without blocking", async () => {
    const user = userEvent.setup();
    renderEditor();
    await user.selectOptions(screen.getByLabelText("Type"), "creditCard");
    await user.type(screen.getByLabelText("Card number"), "4111 1111 1111 1112");
    expect(screen.getByText("This number doesn't pass the card check. Check it for typos.")).toBeTruthy();
    expect(screen.getByLabelText("Card brand")).toHaveProperty("value", "Visa");
  });

  it("blocks an expiry it cannot store", async () => {
    const user = userEvent.setup();
    renderEditor();
    await user.selectOptions(screen.getByLabelText("Type"), "creditCard");
    await user.type(screen.getByLabelText("Expires"), "13/29");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Enter the expiry as MM/YY, for example 04/29.")).toBeTruthy();
  });

  it("names each secret toggle after its field", async () => {
    const user = userEvent.setup();
    renderEditor();
    await user.selectOptions(screen.getByLabelText("Type"), "creditCard");
    const number = screen.getByRole("button", { name: "Reveal Card number" });
    expect(screen.getByRole("button", { name: "Reveal Security code" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reveal PIN" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Show password" })).toBeNull();
    expect(number.getAttribute("aria-controls")).toBe(screen.getByLabelText("Card number").id);
    await user.click(number);
    expect(screen.getByRole("button", { name: "Hide Card number" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByLabelText("Card number").getAttribute("type")).toBe("text");
  });

  it("asks for a title instead of saving an untitled item", async () => {
    const user = userEvent.setup();
    renderEditor();
    expect(screen.queryByText("Give this item a title.")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect((await screen.findByText("Give this item a title.")).textContent).toBe("Give this item a title.");
    expect(screen.getByLabelText("Title").getAttribute("aria-invalid")).toBe("true");
    await user.type(screen.getByLabelText("Title"), "Router");
    expect(screen.queryByText("Give this item a title.")).toBeNull();
  });

  it("warns about a card number that fails the check as a status with an icon", async () => {
    const user = userEvent.setup();
    renderEditor();
    await user.selectOptions(screen.getByLabelText("Type"), "creditCard");
    await user.type(screen.getByLabelText("Card number"), "4111 1111 1111 1112");
    const hint = screen.getByText("This number doesn't pass the card check. Check it for typos.");
    expect(hint.getAttribute("role")).toBe("status");
    expect(hint.querySelector("svg")).not.toBeNull();
  });

  it("does not offer an SSH passphrase to the password manager and lists the key first", async () => {
    const user = userEvent.setup();
    renderEditor();
    await user.selectOptions(screen.getByLabelText("Type"), "sshKey");
    expect(screen.getByLabelText("Passphrase").getAttribute("autocomplete")).toBe("off");
    const key = screen.getByLabelText("Private key");
    const passphrase = screen.getByLabelText("Passphrase");
    expect(key.compareDocumentPosition(passphrase) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("asks for the private key of an SSH key item", async () => {
    const user = userEvent.setup();
    renderEditor();
    await user.selectOptions(screen.getByLabelText("Type"), "sshKey");
    expect(screen.getByLabelText("Private key").tagName).toBe("TEXTAREA");
    expect(screen.getByLabelText("Passphrase")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText(/This isn't an OpenSSH private key/)).toBeTruthy();
  });

  it("has no kind picker when editing", () => {
    renderEditing(stored());
    expect(screen.queryByLabelText("Type")).toBeNull();
    expect(screen.queryByLabelText("Vault")).toBeNull();
    expect(screen.getByLabelText("User name")).toBeTruthy();
  });

  it("blocks changing the kind of a hidden field without a value", async () => {
    const user = userEvent.setup();
    const sent = renderEditing(stored());
    await user.selectOptions(screen.getByLabelText("Field type"), "text");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Enter a value to change the type of a hidden field.")).toBeTruthy();
    expect(sent.some((each) => each.command === "update_item")).toBe(false);
  });

  it("renames a hidden field without sending its value", async () => {
    const user = userEvent.setup();
    const sent = renderEditing(stored());
    const label = screen.getByLabelText("Field name");
    await user.clear(label);
    await user.type(label, "Gate code");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => {
      expect(sent.some((each) => each.command === "update_item")).toBe(true);
    });
    const draft = (sent.find((each) => each.command === "update_item")?.request as { draft: ItemDraftLike }).draft;
    expect(draft.customFields).toEqual([
      { op: "update", elementId: "c1", label: "Gate code", kind: "hidden", value: null },
    ]);
  });

  it("names each custom field row for screen readers", async () => {
    const user = userEvent.setup();
    renderEditing(stored({ customFields: [] }));
    const add = screen.getByRole("button", { name: "Add custom field" });
    await user.click(add);
    await user.click(add);
    expect(screen.getByRole("group", { name: "Custom field 1" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Custom field 2" })).toBeTruthy();
    await user.type(within(screen.getByRole("group", { name: "Custom field 2" })).getByLabelText("Field name"), "Gate");
    expect(screen.getByRole("group", { name: "Gate" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Custom field 1" })).toBeTruthy();
  });

  it("adds a custom field and marks a stored secret for removal", async () => {
    const user = userEvent.setup();
    const sent = renderEditing(stored({ customFields: [] }));
    await user.click(screen.getByRole("button", { name: "Add custom field" }));
    await user.type(screen.getByLabelText("Field name"), "Door code");
    await user.selectOptions(screen.getByLabelText("Field type"), "hidden");
    await user.type(screen.getByLabelText("Value"), "1234");
    await user.click(screen.getByRole("button", { name: "Remove Password" }));
    expect(screen.getByText("Password will be removed when you save.")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => {
      expect(sent.some((each) => each.command === "update_item")).toBe(true);
    });
    const draft = (sent.find((each) => each.command === "update_item")?.request as { draft: ItemDraftLike }).draft;
    expect(draft.customFields).toEqual([{ op: "add", label: "Door code", kind: "hidden", value: "1234" }]);
    expect(draft.secrets).toEqual([{ field: "password", value: null }]);
  });
});

interface ItemDraftLike {
  customFields: unknown[];
  secrets: unknown[];
}
