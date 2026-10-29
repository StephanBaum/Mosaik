import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import type { AppStatus } from "../ipc/contract/index.ts";
import { createMockIpc } from "../ipc/mock.ts";
import { PlatformContext } from "../platform/platform.ts";

/** The mock with a different `app_status`. */
function withStatus(ipc: MosaicIpc, status: AppStatus): MosaicIpc {
  return {
    subscribe: (listener) => ipc.subscribe(listener),
    call: (command, request) =>
      command === "app_status"
        ? Promise.resolve({ ok: true, value: status })
        : ipc.call(command, request),
  };
}

describe("settings", () => {
  it("names the quick-search shortcut of the platform in the conflict hint", async () => {
    const user = userEvent.setup();
    render(
      <PlatformContext value="mac">
        <App ipc={withStatus(createMockIpc({ start: "unlocked" }), { shortcut: "conflict", screenLockDetection: true })} languages={["en"]} />
      </PlatformContext>,
    );
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(await screen.findByText(/⌥⌘Space/)).toBeTruthy();
  });

  it("keeps the last lock trigger that can fire on", async () => {
    const user = userEvent.setup();
    const ipc = withStatus(createMockIpc({ start: "unlocked" }), {
      shortcut: "registered",
      screenLockDetection: false,
    });
    render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.selectOptions(await screen.findByLabelText("Lock after inactivity"), "never");
    await user.type(screen.getByLabelText("Master password"), "mosaic");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    await screen.findByText("Settings saved.");

    // Screen lock cannot be detected here, so sleep is the only trigger left.
    await user.click(screen.getByRole("switch", { name: "Lock when the computer sleeps" }));
    expect(await screen.findByText(/Keep at least one lock on/)).toBeTruthy();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole<HTMLInputElement>("switch", { name: "Lock when the computer sleeps" }).checked).toBe(true);
  });

  it("asks for the master password before turning auto-lock off", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: "Settings" }));

    const autoLock = await screen.findByLabelText("Lock after inactivity");
    await user.selectOptions(autoLock, "never");
    expect(await screen.findByRole("dialog", { name: "Confirm with your master password" })).toBeTruthy();
    await user.type(screen.getByLabelText("Master password"), "wrong");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect((await screen.findAllByRole("alert")).some((a) => /master password/i.test(a.textContent))).toBe(true);

    await user.type(screen.getByLabelText("Master password"), "mosaic");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(await screen.findByText("Settings saved.")).toBeTruthy();
    expect(screen.getByLabelText<HTMLSelectElement>("Lock after inactivity").value).toBe("never");
  });

  it("switches the SSH agent on and shows the endpoint", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: "Settings" }));
    await user.click(await screen.findByRole("switch", { name: "Use my SSH keys from Mosaic" }));
    expect(await screen.findByText(/agent\.sock/)).toBeTruthy();
  });
});
