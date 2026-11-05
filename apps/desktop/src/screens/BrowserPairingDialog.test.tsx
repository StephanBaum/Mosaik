import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App.tsx";
import { ALLOW_GUARD_MS } from "../hooks/useAllowGuard.ts";
import { createMockIpc } from "../ipc/mock.ts";
import { formatCode } from "./BrowserPairingDialog.tsx";

afterEach(() => {
  vi.useRealTimers();
});

const NAME = "A browser wants to connect to Mosaic";

describe("browser pairing dialog", () => {
  it("formats the code as two groups of three", () => {
    expect(formatCode("4F7K2Q")).toBe("4F7-K2Q");
    expect(formatCode("short")).toBe("short");
  });

  it("shows the code and the warning, focuses Deny and keeps Allow disabled for one second", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", browserPair: true })} languages={["en"]} />);

    const dialog = await screen.findByRole("dialog", { name: NAME }, { timeout: 3000 });
    const deny = within(dialog).getByRole("button", { name: "Deny" });
    await waitFor(() => {
      expect(document.activeElement).toBe(deny);
    });
    expect(within(dialog).getByText("4F7-K2Q")).toBeTruthy();
    expect(within(dialog).getByText(/Only allow this if you just clicked Connect/)).toBeTruthy();
    const allow = within(dialog).getByRole("button", { name: "Allow" });
    expect((allow as HTMLButtonElement).disabled).toBe(true);
    // The label starts as the browser the extension reported.
    const label = within(dialog).getByRole("textbox", { name: "Name for this browser" });
    expect((label as HTMLInputElement).value).toBe("Edge");

    await waitFor(
      () => {
        expect((allow as HTMLButtonElement).disabled).toBe(false);
      },
      { timeout: ALLOW_GUARD_MS + 1500 },
    );
    await user.click(allow);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: NAME })).toBeNull();
    });
  });

  it("pairs under the name the user typed", async () => {
    const user = userEvent.setup();
    const ipc = createMockIpc({ start: "unlocked", browserPair: true });
    render(<App ipc={ipc} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: NAME }, { timeout: 3000 });
    const field = within(dialog).getByRole("textbox", { name: "Name for this browser" });
    await user.clear(field);
    await user.type(field, "Work laptop");
    const allow = within(dialog).getByRole("button", { name: "Allow" });
    await waitFor(
      () => {
        expect((allow as HTMLButtonElement).disabled).toBe(false);
      },
      { timeout: ALLOW_GUARD_MS + 1500 },
    );
    await user.click(allow);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: NAME })).toBeNull();
    });
    const status = await ipc.call("browser_status", null);
    expect(status.ok && status.value.pairings.map((p) => p.label)).toEqual(["Chrome", "Work laptop"]);
  });

  it("does not allow an empty name, and denies at once", async () => {
    const user = userEvent.setup();
    const ipc = createMockIpc({ start: "unlocked", browserPair: true });
    render(<App ipc={ipc} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: NAME }, { timeout: 3000 });
    await user.clear(within(dialog).getByRole("textbox", { name: "Name for this browser" }));
    await new Promise((resolve) => setTimeout(resolve, ALLOW_GUARD_MS + 300));
    const allow = within(dialog).getByRole("button", { name: "Allow" });
    expect((allow as HTMLButtonElement).disabled).toBe(true);
    await user.click(within(dialog).getByRole("button", { name: "Deny" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: NAME })).toBeNull();
    });
    const status = await ipc.call("browser_status", null);
    expect(status.ok && status.value.pairings).toHaveLength(1);
  });

  it("announces the countdown to screen readers once, in the last ten seconds", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    render(<App ipc={createMockIpc({ start: "unlocked", browserPair: true })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: NAME }, { timeout: 3000 });
    const live = within(dialog).getByRole("status");
    expect(live.textContent).toBe("");
    // The request lives 120 s; move on in steps until the announcement appears.
    for (let elapsed = 0; elapsed < 125_000 && live.textContent === ""; elapsed += 5_000) {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });
    }
    expect(live.textContent).toMatch(/^Closes automatically in \d+ seconds?$/);
  });
});
