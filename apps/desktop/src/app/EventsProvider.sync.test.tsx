import { act, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import type { AppEvent, CommandName, RequestOf } from "../ipc/contract/index.ts";
import { createMockIpc } from "../ipc/mock.ts";

function harness() {
  const mock = createMockIpc({ start: "unlocked" });
  const listeners: ((event: AppEvent) => void)[] = [];
  let listCalls = 0;
  const ipc: MosaicIpc = {
    subscribe: (listener) => {
      listeners.push(listener);
      return mock.subscribe(listener);
    },
    call: <K extends CommandName>(command: K, request: RequestOf<K>) => {
      if (command === "list_items") {
        listCalls += 1;
      }
      return mock.call(command, request);
    },
  };
  const emit = (event: AppEvent) => {
    act(() => {
      for (const listener of listeners) {
        listener(event);
      }
    });
  };
  return { ipc, emit, listCalls: () => listCalls };
}

describe("sync events", () => {
  it("reloads the vault lists when a sync run changed vaults", async () => {
    const { ipc, emit, listCalls } = harness();
    render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    const before = listCalls();
    emit({ type: "syncDataChanged", vaultIds: ["019a2b3c-4d5e-7010-8000-000000000001"] });
    await waitFor(() => {
      expect(listCalls()).toBeGreaterThan(before);
    });
  });

  it("tells the user when a sync run merged or repaired items", async () => {
    const { ipc, emit } = harness();
    render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    emit({ type: "syncAttention", conflicts: 2, restored: 0, replacedUnreadable: 1 });
    expect(await screen.findByText(/Sync merged or repaired some items/)).toBeTruthy();
  });

  it("shows the state a sync event reports in the sidebar", async () => {
    const { ipc, emit } = harness();
    render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    emit({ type: "syncStateChanged", state: { status: "signedOut", lastSyncedAt: null, error: "syncSignedOut" } });
    await waitFor(() => {
      expect(document.querySelector(".sidebar__sync-text")?.textContent).toBe("Signed out");
    });
  });
});
