import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "../../App.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { AppEvent, ImportDetected } from "../../ipc/contract/index.ts";
import { createMockIpc, type MockOptions } from "../../ipc/mock.ts";

type User = ReturnType<typeof userEvent.setup>;

const chromium: ImportDetected = { source: "chromium", format: "csv", syncedFolder: null };

async function openImport(user: User) {
  await screen.findByText("GitHub");
  await user.keyboard("{Control>}k{/Control}");
  await user.click(await screen.findByRole("option", { name: "Import from another app" }));
  await screen.findByRole("heading", { name: "Import" });
}

async function startWith(options: Partial<MockOptions>, source: string) {
  const user = userEvent.setup();
  render(<App ipc={createMockIpc({ start: "unlocked", ...options })} languages={["en"]} />);
  await openImport(user);
  await user.click(screen.getByRole("button", { name: source }));
  await user.click(screen.getByRole("button", { name: "Choose the export file" }));
  return user;
}

async function previewAndCommit(user: User) {
  await user.click(await screen.findByRole("button", { name: "Preview" }));
  await screen.findByRole("checkbox", { name: /GitHub/ });
  await user.click(screen.getByRole("button", { name: "Import 1 item" }));
}

describe("import flow", () => {
  it("shows browsers and managers by name without logos", async () => {
    const user = userEvent.setup();
    const { container } = render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openImport(user);
    for (const name of ["Chrome", "LastPass", "Other CSV or file"]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }
    expect(container.querySelector("img")).toBeNull();
  });

  it("guides Chrome and copies its internal address", async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText");
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openImport(user);
    await user.click(screen.getByRole("button", { name: "Chrome" }));
    expect(screen.getByText("chrome://password-manager/settings")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Copy address" }));
    expect(writeText).toHaveBeenCalledWith("chrome://password-manager/settings");
  });

  it("shows a mismatch notice when the file is another product's", async () => {
    await startWith({ importDetected: { source: "enpass", format: "enpass", syncedFolder: null } }, "Chrome");
    expect(await screen.findByText(/This file looks like an export from Enpass\./)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Preview" })).toBeTruthy();
  });

  it("refuses an unrecognised file before the preview", async () => {
    await startWith({ importDetected: { source: null, format: null, syncedFolder: null } }, "Chrome");
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Preview" })).toBeNull();
    expect(screen.getByRole("button", { name: "Choose the export file" })).toBeTruthy();
  });

  it("asks for the password of a KeePass database", async () => {
    await startWith({ importDetected: { source: "keepass", format: "keepass", syncedFolder: null } }, "KeePass");
    expect(await screen.findByLabelText("Password of the KeePass database")).toBeTruthy();
  });

  it("lists what won't come across", async () => {
    await startWith({ importDetected: chromium }, "Chrome");
    await userEvent.setup().click(await screen.findByRole("button", { name: "Preview" }));
    expect(await screen.findByRole("heading", { name: "Won't come across" })).toBeTruthy();
    expect(screen.getByText(/Passkeys: Chrome doesn't put them in its export/)).toBeTruthy();
  });

  it("commits and offers to delete the export", async () => {
    const user = await startWith({}, "Enpass");
    await previewAndCommit(user);
    await user.click(await screen.findByRole("button", { name: "Delete enpass-export.json now" }));
    expect(await screen.findByText(/enpass-export\.json was overwritten and deleted\./)).toBeTruthy();
    expect(screen.getByText(/older copies can survive/)).toBeTruthy();
  });

  it("cleanup reports a failed delete", async () => {
    const user = await startWith({ importDeleteFails: true }, "Enpass");
    await previewAndCommit(user);
    await user.click(await screen.findByRole("button", { name: "Delete enpass-export.json now" }));
    expect(await screen.findByText(/enpass-export\.json couldn't be deleted/)).toBeTruthy();
    expect(screen.queryByText(/was overwritten and deleted/)).toBeNull();
  });

  it("names a synced folder and shows the checklist", async () => {
    const detected: ImportDetected = { ...chromium, syncedFolder: "oneDrive" };
    const user = await startWith({ importDetected: detected }, "Chrome");
    await previewAndCommit(user);
    expect(await screen.findByText(/This file is in your OneDrive folder/)).toBeTruthy();
    expect(screen.getByText(/Turn off password saving and auto sign-in in Chrome/)).toBeTruthy();
  });

  it("cancels and drops the preview", async () => {
    const user = await startWith({}, "Enpass");
    await user.click(await screen.findByRole("button", { name: "Preview" }));
    await screen.findByRole("checkbox", { name: /GitHub/ });
    await user.click(screen.getByRole("button", { name: "Cancel import" }));
    expect(await screen.findByText("GitHub")).toBeTruthy();
  });

  it("takes a dropped file on the steps screen", async () => {
    const user = userEvent.setup();
    const mock = createMockIpc({ start: "unlocked" });
    const listeners: ((event: AppEvent) => void)[] = [];
    const ipc: MosaicIpc = {
      subscribe: (listener) => {
        listeners.push(listener);
        return mock.subscribe(listener);
      },
      call: (command, request) => mock.call(command, request),
    };
    render(<App ipc={ipc} languages={["en"]} />);
    await openImport(user);
    await user.click(screen.getByRole("button", { name: "Enpass" }));
    act(() => {
      for (const listener of listeners) {
        listener({ type: "importFileDropped", fileToken: "file-1", fileName: "dropped.json" });
      }
    });
    expect(await screen.findByText(/File: dropped\.json/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Preview" })).toBeTruthy();
  });
});
