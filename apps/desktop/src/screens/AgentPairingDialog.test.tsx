import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import { ALLOW_GUARD_MS } from "../hooks/useAllowGuard.ts";
import type { MosaicIpc } from "../ipc/client.ts";
import type { AppEvent, CommandName } from "../ipc/contract/index.ts";
import { createMockIpc } from "../ipc/mock.ts";

const TITLE = "An agent wants to connect";

describe("agent pairing dialog", () => {
  it("code and name are shown", async () => {
    render(<App ipc={createMockIpc({ start: "unlocked", agent: "pair" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: TITLE }, { timeout: 3000 });
    expect(within(dialog).getByText("K7M-2QX")).toBeTruthy();
    expect(within(dialog).getByText("Cursor")).toBeTruthy();
    expect(within(dialog).getByText(/Only allow this if you just ran .*mosaic agent pair.* in a terminal/)).toBeTruthy();
    expect(within(dialog).getByText(/Mosaic cannot check the name/)).toBeTruthy();
    const deny = within(dialog).getByRole("button", { name: "Deny" });
    await waitFor(() => {
      expect(document.activeElement).toBe(deny);
    });
  });

  it("keeps Connect disabled for one second, then pairs", async () => {
    const base = createMockIpc({ start: "unlocked", agent: "pair" });
    const sent: { command: CommandName; request: unknown }[] = [];
    const ipc: MosaicIpc = {
      subscribe: (listener) => base.subscribe(listener),
      call: (command, request) => {
        sent.push({ command, request });
        return base.call(command, request);
      },
    };
    const user = userEvent.setup();
    render(<App ipc={ipc} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: TITLE }, { timeout: 3000 });
    const connect = within(dialog).getByRole("button", { name: "Connect" });
    expect((connect as HTMLButtonElement).disabled).toBe(true);
    await waitFor(
      () => {
        expect((connect as HTMLButtonElement).disabled).toBe(false);
      },
      { timeout: ALLOW_GUARD_MS + 1500 },
    );
    await user.click(connect);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: TITLE })).toBeNull();
    });
    expect(sent.filter((each) => each.command === "agent_pairing_respond")).toEqual([
      {
        command: "agent_pairing_respond",
        request: { requestId: "a1b2c3d4e5f60718293a4b5c6d7e8f90", decision: "allow" },
      },
    ]);
  });

  it("denies on Esc", async () => {
    render(<App ipc={createMockIpc({ start: "unlocked", agent: "pair" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: TITLE }, { timeout: 3000 });
    // Esc is the dialog's native cancel event (jsdom does not make one from the key).
    fireEvent(dialog, new Event("cancel", { cancelable: true }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: TITLE })).toBeNull();
    });
  });

  it("the in-use indicator follows the activity events and says nothing otherwise", async () => {
    const base = createMockIpc({ start: "unlocked" });
    const listeners = new Set<(event: AppEvent) => void>();
    const push = (inFlight: number) => {
      act(() => {
        for (const listener of listeners) {
          listener({ type: "agentActivity", inFlight });
        }
      });
    };
    const ipc: MosaicIpc = {
      subscribe: (listener) => {
        listeners.add(listener);
        return base.subscribe(listener);
      },
      call: (command, request) => base.call(command, request),
    };
    render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    expect(screen.queryByText("An agent is using a key")).toBeNull();
    expect(document.querySelector('.agent-indicator[aria-live="polite"]')).not.toBeNull();
    push(1);
    expect(await screen.findByText("An agent is using a key")).toBeTruthy();
    push(0);
    await waitFor(() => {
      expect(screen.queryByText("An agent is using a key")).toBeNull();
    });
  });
});
