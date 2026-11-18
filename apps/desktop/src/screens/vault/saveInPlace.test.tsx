import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../../App.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { CommandName, RequestOf } from "../../ipc/contract/index.ts";
import { createMockIpc } from "../../ipc/mock.ts";

/** The mock IPC with a count of the `list_items` calls. */
function harness() {
  const mock = createMockIpc({ start: "unlocked" });
  let listCalls = 0;
  const ipc: MosaicIpc = {
    subscribe: (listener) => mock.subscribe(listener),
    call: <K extends CommandName>(command: K, request: RequestOf<K>) => {
      if (command === "list_items") {
        listCalls += 1;
      }
      return mock.call(command, request);
    },
  };
  return { ipc, listCalls: () => listCalls };
}

describe("saving an item", () => {
  it("updates the list in place and fetches no list again", async () => {
    const user = userEvent.setup();
    const { ipc, listCalls } = harness();
    render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    // The sidebar counts come from the list's own result: one list request in all.
    const before = listCalls();
    await user.click(screen.getByRole("button", { name: "New item" }));
    await user.type(await screen.findByLabelText("Title"), "Zed login");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect((await screen.findAllByText("Zed login")).length).toBeGreaterThan(0);
    expect(listCalls()).toBe(before);
  });
});
