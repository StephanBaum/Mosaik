import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { catalogs } from "@mosaic/i18n";
import { describe, expect, it } from "vitest";
import { App } from "../../App.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { CommandName, RequestOf } from "../../ipc/contract/index.ts";
import { createMockIpc } from "../../ipc/mock.ts";
import type { MockSyncOptions } from "../../ipc/mock/syncData.ts";
import { sampleAccount } from "../../ipc/mock/data.ts";

type User = ReturnType<typeof userEvent.setup>;

async function openJoin(ipc: MosaicIpc) {
  const user = userEvent.setup();
  render(<App ipc={ipc} languages={["en"]} />);
  await user.click(await screen.findByRole("button", { name: "I already use Mosaic" }));
  await screen.findByRole("heading", { name: "Add this device to your account" });
  return user;
}

async function fillSecrets(user: User, password = "mosaic") {
  await user.type(screen.getByLabelText("Master password"), password);
  await user.type(screen.getByLabelText("Secret Key"), sampleAccount.secretKey);
}

function joining(sync: MockSyncOptions = {}) {
  return createMockIpc({ sync });
}

describe("join an existing account", () => {
  it("joins through a Mosaic server and opens the vault", async () => {
    const user = await openJoin(joining());
    // Two choices, the server first.
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((tab) => tab.textContent)).toEqual(["Mosaic server", "Cloud folder"]);
    expect(tabs[0]?.getAttribute("aria-selected")).toBe("true");
    await user.type(screen.getByLabelText("Server address"), "https://sync.example.com");
    await user.type(screen.getByLabelText("Sync account email"), "alex@example.com");
    expect(screen.getByLabelText<HTMLInputElement>("Name of this device").value).toMatch(/^Mosaic on /);
    await fillSecrets(user);
    await user.click(screen.getByRole("button", { name: "Add this device" }));
    // The joined account is kit-verified: no Emergency Kit steps, straight into the vault.
    expect(await screen.findByText("GitHub")).toBeTruthy();
    expect(screen.queryByRole("progressbar")).toBeNull();
  });

  it("shows a wrong password or Secret Key and keeps the form", async () => {
    const user = await openJoin(joining());
    await user.type(screen.getByLabelText("Server address"), "https://sync.example.com");
    await user.type(screen.getByLabelText("Sync account email"), "alex@example.com");
    await fillSecrets(user, "wrong");
    await user.click(screen.getByRole("button", { name: "Add this device" }));
    expect(
      (await screen.findAllByRole("alert")).some((a) => a.textContent === catalogs.en["error.wrongCredentials"]),
    ).toBe(true);
    expect(screen.getByLabelText("Server address")).toBeTruthy();
  });

  it("asks for the two-step code after the password", async () => {
    const user = await openJoin(joining({ join2fa: true }));
    await user.type(screen.getByLabelText("Server address"), "https://sync.example.com");
    await user.type(screen.getByLabelText("Sync account email"), "alex@example.com");
    await fillSecrets(user);
    await user.click(screen.getByRole("button", { name: "Add this device" }));
    await screen.findByRole("heading", { name: "Enter your code" });
    await user.type(screen.getByLabelText("6-digit code"), "000000");
    await user.click(screen.getByRole("button", { name: "Verify" }));
    expect((await screen.findAllByRole("alert")).some((a) => /code isn't right/.test(a.textContent))).toBe(true);
    await user.type(screen.getByLabelText("6-digit code"), "123456");
    await user.click(screen.getByRole("button", { name: "Verify" }));
    expect(await screen.findByText("GitHub")).toBeTruthy();
  });

  it("joins from a cloud folder", async () => {
    const user = await openJoin(joining({ folder: "mosaic" }));
    await user.click(screen.getByRole("tab", { name: "Cloud folder" }));
    expect(screen.queryByLabelText("Server address")).toBeNull();
    const submit = screen.getByRole("button", { name: "Add this device" });
    expect((submit as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Choose folder…" }));
    expect(await screen.findByText("C:\\Users\\alex\\Dropbox\\Mosaic")).toBeTruthy();
    // One account in the folder: nothing to pick.
    expect(screen.queryByLabelText("Account")).toBeNull();
    await fillSecrets(user);
    await user.click(screen.getByRole("button", { name: "Add this device" }));
    expect(await screen.findByText("GitHub")).toBeTruthy();
  });

  it("lets the user pick the account when the folder lists more than one", async () => {
    const inner = joining({ folder: "mosaic" });
    const calls: unknown[] = [];
    const ipc: MosaicIpc = {
      subscribe: (listener) => inner.subscribe(listener),
      call: async <K extends CommandName>(command: K, request: RequestOf<K>) => {
        if (command === "sync_pick_folder") {
          const result = await inner.call("sync_pick_folder", null);
          if (result.ok && result.value !== null) {
            return {
              ok: true,
              value: { ...result.value, accounts: ["019a2b3c-4d5e-7030-8000-00000000ffff", ...result.value.accounts] },
            } as never;
          }
        }
        if (command === "join_folder") {
          calls.push(request);
        }
        return inner.call(command, request);
      },
    };
    const user = await openJoin(ipc);
    await user.click(screen.getByRole("tab", { name: "Cloud folder" }));
    await user.click(screen.getByRole("button", { name: "Choose folder…" }));
    const select = await screen.findByLabelText<HTMLSelectElement>("Account");
    expect(within(select).getAllByRole("option")).toHaveLength(2);
    await user.selectOptions(select, sampleAccount.accountId);
    await fillSecrets(user);
    await user.click(screen.getByRole("button", { name: "Add this device" }));
    expect(await screen.findByText("GitHub")).toBeTruthy();
    expect(calls).toMatchObject([{ accountId: sampleAccount.accountId }]);
  });

  it("says when the folder holds no Mosaic account", async () => {
    const user = await openJoin(joining({ folder: "empty" }));
    await user.click(screen.getByRole("tab", { name: "Cloud folder" }));
    await user.click(screen.getByRole("button", { name: "Choose folder…" }));
    expect(await screen.findByText(/This folder holds no Mosaic account/)).toBeTruthy();
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Add this device" }).disabled).toBe(true);
  });

  it("goes back to creating an account", async () => {
    const user = await openJoin(joining());
    await user.click(screen.getByRole("button", { name: "Back to create account" }));
    expect(await screen.findByRole("heading", { name: "Create your account" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "I already use Mosaic" })).toBeTruthy();
  });
});
