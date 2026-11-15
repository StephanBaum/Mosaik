import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "../../App.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { CommandName, RequestOf, SetupCodeShown } from "../../ipc/contract/index.ts";
import { createMockIpc } from "../../ipc/mock.ts";
import type { MockSyncOptions } from "../../ipc/mock/syncData.ts";

/** Counts the dark modules the dialog draws (jsdom has no canvas). */
const drawn = { fills: 0 };

beforeEach(() => {
  drawn.fills = 0;
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation((() => ({
    fillStyle: "",
    fillRect: () => {
      drawn.fills += 1;
    },
  })) as unknown as HTMLCanvasElement["getContext"]);
});

afterEach(() => {
  vi.restoreAllMocks();
});

/** The mock, with the lifetime of the code shortened so the 60 s end can be waited for. */
function shortLived(sync: MockSyncOptions, lifetimeMs: number): MosaicIpc {
  const inner = createMockIpc({ start: "unlocked", sync });
  return {
    subscribe: (listener) => inner.subscribe(listener),
    call: async <K extends CommandName>(command: K, request: RequestOf<K>) => {
      const result = await inner.call(command, request);
      if (command === "setup_code_show" && result.ok) {
        const shown = result.value as SetupCodeShown;
        return { ok: true, value: { ...shown, expiresAt: Date.now() + lifetimeMs } } as never;
      }
      return result;
    },
  };
}

async function openDevices(sync: MockSyncOptions, ipc?: MosaicIpc) {
  const user = userEvent.setup();
  render(<App ipc={ipc ?? createMockIpc({ start: "unlocked", sync })} languages={["en"]} />);
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: "Settings" }));
  const heading = await screen.findByRole("heading", { name: "Set up a phone" });
  return { user, section: heading.closest("section") as HTMLElement };
}

async function showCode(user: ReturnType<typeof userEvent.setup>, password = "mosaic") {
  await user.click(screen.getByRole("button", { name: "Set up a phone" }));
  const reauth = within(screen.getByRole("dialog"));
  await user.type(reauth.getByLabelText("Master password"), password);
  await user.click(reauth.getByRole("button", { name: "Confirm" }));
}

describe("set up a phone", () => {
  it("asks for the master password, then draws the code", async () => {
    const { user } = await openDevices({ variant: "hosted" });
    await showCode(user);
    const code = await screen.findByRole("img", { name: "Setup code for your phone" });
    expect(code.tagName).toBe("CANVAS");
    // The paper, then one square per dark module.
    expect(drawn.fills).toBeGreaterThan(100);
  });

  it("keeps the code away on a wrong password", async () => {
    const { user } = await openDevices({ variant: "hosted" });
    await showCode(user, "wrong");
    expect((await screen.findAllByRole("alert")).length).toBeGreaterThan(0);
    expect(screen.queryByRole("img", { name: "Setup code for your phone" })).toBeNull();
  });

  it("hides the code when the window loses focus", async () => {
    const { user, section } = await openDevices({ variant: "hosted" });
    await showCode(user);
    await screen.findByRole("img", { name: "Setup code for your phone" });
    fireEvent.blur(window);
    await waitFor(() => {
      expect(screen.queryByRole("img", { name: "Setup code for your phone" })).toBeNull();
    });
    expect(within(section).getByText("You left this window, so the code is hidden.")).toBeTruthy();
    expect(within(section).getByRole("button", { name: "Show again" })).toBeTruthy();
  });

  it("hides the code when the vault locks", async () => {
    const ipc = createMockIpc({ start: "unlocked", sync: { variant: "hosted" } });
    const { user } = await openDevices({ variant: "hosted" }, ipc);
    await showCode(user);
    await screen.findByRole("img", { name: "Setup code for your phone" });
    await ipc.call("lock", null);
    await waitFor(() => {
      expect(screen.queryByRole("img", { name: "Setup code for your phone" })).toBeNull();
    });
    expect(screen.queryByRole("button", { name: "Show again" })).toBeNull();
  });

  it("hides the code when the time is up", async () => {
    const ipc = shortLived({ variant: "hosted" }, 60);
    const { user, section } = await openDevices({ variant: "hosted" }, ipc);
    await showCode(user);
    await waitFor(() => {
      expect(within(section).getByText("The 60 seconds are over, so the code is hidden.")).toBeTruthy();
    });
    expect(screen.queryByRole("img", { name: "Setup code for your phone" })).toBeNull();
  });

  it("says what is supported when the account syncs through a folder", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", sync: { variant: "folder" } })} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(
      await screen.findByText("This account syncs through a folder. The phone apps support Mosaic sync for now."),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Set up a phone" })).toBeNull();
  });
});
