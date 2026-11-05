import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import type { BrowserStatus } from "../ipc/contract/index.ts";
import { createMockIpc } from "../ipc/mock.ts";

async function openSettings() {
  const user = userEvent.setup();
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: "Settings" }));
  return user;
}

describe("browser settings", () => {
  it("lists the paired browsers and disconnects one after a confirmation", async () => {
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    const user = await openSettings();
    const heading = await screen.findByRole("heading", { name: "Browsers" });
    const section = heading.closest("section") as HTMLElement;
    expect(await within(section).findByText("Chrome")).toBeTruthy();
    expect(within(section).getByText(/Chromium-based browser/)).toBeTruthy();

    await user.click(within(section).getByRole("button", { name: "Disconnect Chrome" }));
    expect(within(section).getByText("Disconnect “Chrome”?")).toBeTruthy();
    // Cancel keeps it.
    await user.click(within(section).getByRole("button", { name: "Cancel" }));
    expect(within(section).getByText("Chrome")).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: "Disconnect Chrome" }));
    await user.click(within(section).getByRole("button", { name: "Disconnect" }));
    await waitFor(() => {
      expect(within(section).queryByText("Chrome")).toBeNull();
    });
    expect(within(section).getByText(/No browser is connected yet/)).toBeTruthy();
  });

  it("shows when the connection for browsers could not start", async () => {
    const base = createMockIpc({ start: "unlocked" });
    const status: BrowserStatus = { endpoint: "inUse", registry: "ok", pairings: [] };
    const ipc: MosaicIpc = {
      subscribe: (listener) => base.subscribe(listener),
      call: (command, request) =>
        command === "browser_status"
          ? Promise.resolve({ ok: true, value: status })
          : base.call(command, request),
    };
    render(<App ipc={ipc} languages={["en"]} />);
    await openSettings();
    expect(await screen.findByText(/already uses the connection for browsers/)).toBeTruthy();
  });

  it("says when the saved browser connections can't be read", async () => {
    const base = createMockIpc({ start: "unlocked" });
    const status: BrowserStatus = { endpoint: "running", registry: "unreadable", pairings: [] };
    const ipc: MosaicIpc = {
      subscribe: (listener) => base.subscribe(listener),
      call: (command, request) =>
        command === "browser_status"
          ? Promise.resolve({ ok: true, value: status })
          : base.call(command, request),
    };
    render(<App ipc={ipc} languages={["en"]} />);
    await openSettings();
    expect(await screen.findByText(/Saved browser connections can't be read/)).toBeTruthy();
  });

  it("reads the list again when Rust reports a change", async () => {
    const ipc = createMockIpc({ start: "unlocked", browserPair: true });
    render(<App ipc={ipc} languages={["en"]} />);
    const user = userEvent.setup();
    const dialog = await screen.findByRole(
      "dialog",
      { name: "A browser wants to connect to Mosaic" },
      { timeout: 3000 },
    );
    const allow = within(dialog).getByRole("button", { name: "Allow" });
    await waitFor(
      () => {
        expect((allow as HTMLButtonElement).disabled).toBe(false);
      },
      { timeout: 2500 },
    );
    await user.click(allow);
    await user.click(await screen.findByRole("button", { name: "Settings" }));
    expect(await screen.findByText("Edge")).toBeTruthy();
  });
});
