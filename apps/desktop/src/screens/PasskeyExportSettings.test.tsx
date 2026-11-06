import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import type {
  CommandName,
  ExportPasskeysRequest,
  ItemSummary,
  ListItemsRequest,
  PasskeysExported,
  RequestOf,
} from "../ipc/contract/index.ts";
import { createMockIpc } from "../ipc/mock.ts";

/**
 * The mock with one passkey in every vault listing and a scripted export: records the calls,
 * checks the master password like the backend does, or cancels the save dialog.
 */
function withExport(cancel = false, empty = false) {
  const inner = createMockIpc({ start: "unlocked" });
  const calls: unknown[] = [];
  const ipc: MosaicIpc = {
    subscribe: (listener) => inner.subscribe(listener),
    call: async <K extends CommandName>(command: K, request: RequestOf<K>) => {
      if (command === "list_items") {
        const listed = await inner.call("list_items", request as ListItemsRequest);
        const first: ItemSummary | undefined = listed.ok ? listed.value[0] : undefined;
        const listing = listed.ok ? listed.value : [];
        // `empty`: the vaults hold no passkey (the mock's own one is dropped); the list stays.
        const value: ItemSummary[] = empty
          ? listing.filter((item) => item.kind !== "passkey")
          : first === undefined
            ? []
            : [{ ...first, kind: "passkey" }];
        return { ok: true, value };
      }
      if (command === "export_passkeys_cxf") {
        calls.push(request);
        if ((request as ExportPasskeysRequest).password !== "mosaic") {
          return {
            ok: false,
            error: { code: "wrongCredentials" },
          };
        }
        const value: PasskeysExported = cancel
          ? { kind: "cancelled" }
          : { kind: "saved", count: 2 };
        return { ok: true, value };
      }
      return inner.call(command, request);
    },
  };
  return { ipc, calls };
}

async function openSettings(ipc: MosaicIpc) {
  const user = userEvent.setup();
  render(<App ipc={ipc} languages={["en"]} />);
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: "Settings" }));
  await screen.findByRole("heading", { name: "Export passkeys" });
  return user;
}

async function openExport(user: ReturnType<typeof userEvent.setup>) {
  const open = await screen.findByRole("button", { name: "Export passkeys" });
  await waitEnabled(open);
  await user.click(open);
  return screen.findByRole("dialog");
}

async function waitEnabled(button: HTMLElement) {
  for (let i = 0; i < 50 && (button as HTMLButtonElement).disabled; i += 1) {
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

describe("export passkeys", () => {
  it("shows the plaintext warning as a note above the vault choice", async () => {
    const { ipc } = withExport();
    await openSettings(ipc);
    const note = await screen.findByRole("note");
    expect(note.className).toMatch(/callout--warning/);
    expect(note.textContent).toMatch(/passkeys unencrypted/);
    const vault = screen.getByRole("combobox", { name: "Vault" });
    expect(note.compareDocumentPosition(vault) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.queryByText("This vault has no passkeys to export.")).toBeNull();
  });

  it("says there is nothing to export instead of the warning, and the disabled button points to it", async () => {
    const { ipc } = withExport(false, true);
    await openSettings(ipc);
    const none = await screen.findByText("This vault has no passkeys to export.");
    expect(screen.queryByRole("note")).toBeNull();
    const button = screen.getByRole<HTMLButtonElement>("button", { name: "Export passkeys" });
    expect(button.disabled).toBe(true);
    expect(button.getAttribute("aria-describedby")).toBe(none.id);
  });

  it("warns that the file is unencrypted and asks for the master password", async () => {
    const { ipc, calls } = withExport();
    const user = await openSettings(ipc);
    expect(
      screen.getByText(
        /The file contains your passkeys unencrypted\. Anyone with the file can sign in as you\. Delete it after importing\./,
      ),
    ).toBeTruthy();
    const dialog = await openExport(user);
    expect(calls).toEqual([]);
    expect(dialog.textContent).toMatch(/needs your master password/);

    // A wrong password is refused and the dialog stays open.
    await user.type(screen.getByLabelText("Master password"), "wrong");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(await screen.findByText(/master password or Secret Key didn.t work/)).toBeTruthy();
    expect(screen.queryByText(/exported\./)).toBeNull();

    await user.type(screen.getByLabelText("Master password"), "mosaic");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(
      await screen.findByText(/2 passkeys exported\. Delete the file after importing\./),
    ).toBeTruthy();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(calls).toHaveLength(2);
    expect(calls[1]).toMatchObject({ password: "mosaic" });
  }, 20_000);

  it("closes quietly when the save dialog is cancelled", async () => {
    const { ipc, calls } = withExport(true);
    const user = await openSettings(ipc);
    await openExport(user);
    await user.type(screen.getByLabelText("Master password"), "mosaic");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(calls).toHaveLength(1);
    expect(screen.queryByText(/exported\./)).toBeNull();
  });
});
