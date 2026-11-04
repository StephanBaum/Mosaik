import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SecretsProvider } from "../../app/SecretsContext.tsx";
import { ToastProvider } from "../../components/ToastProvider.tsx";
import { I18nProvider } from "../../i18n/I18nProvider.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { CommandName, IpcError } from "../../ipc/contract/index.ts";
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
