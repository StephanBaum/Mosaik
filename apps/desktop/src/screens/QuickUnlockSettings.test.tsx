import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import { createMockIpc, type MockOptions } from "../ipc/mock.ts";

async function openSettings(options: MockOptions = {}) {
  const user = userEvent.setup();
  const calls: { command: string; request: unknown }[] = [];
  const mock = createMockIpc({ start: "unlocked", ...options });
  const ipc: MosaicIpc = {
    subscribe: (listener) => mock.subscribe(listener),
    call: (command, request) => {
      calls.push({ command, request });
      return mock.call(command, request);
    },
  };
  render(<App ipc={ipc} languages={["en"]} />);
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: "Settings" }));
  await screen.findByRole("heading", { name: "Quick unlock" });
  return { user, calls };
}

describe("settings, quick unlock", () => {
  it("says biometrics are unavailable where the OS has none", async () => {
    await openSettings({ biometricPlatform: null });
    expect(await screen.findByText("Biometric unlock isn't available on Linux.")).toBeTruthy();
    expect(screen.queryByRole("switch", { name: "Windows Hello" })).toBeNull();
  });

  it("hides all security-key controls and says so while keys are unsupported", async () => {
    await openSettings();
    expect(await screen.findByText("Security keys aren't supported on this system yet.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Manage" })).toBeNull();
  });

  it("turns Windows Hello on after the master password and off again", async () => {
    const { user, calls } = await openSettings();
    const toggle = await screen.findByRole("switch", { name: "Windows Hello" });
    await user.click(toggle);

    const dialog = await screen.findByRole("dialog", { name: "Confirm with your master password" });
    await user.type(within(dialog).getByLabelText("Master password"), "mosaic");
    await user.click(within(dialog).getByRole("button", { name: "Confirm" }));
    await screen.findByText(/Mosaic asks for your master password again on/);
    expect(screen.getByRole<HTMLInputElement>("switch", { name: "Windows Hello" }).checked).toBe(true);
    expect(calls.filter((c) => c.command === "enroll_biometric").map((c) => c.request)).toEqual([
      { password: null },
      { password: "mosaic" },
    ]);

    await user.click(screen.getByRole("switch", { name: "Windows Hello" }));
    await screen.findByRole("switch", { name: "Windows Hello", checked: false });
    expect(calls.some((c) => c.command === "remove_biometric")).toBe(true);
  });

  it("lists the registered keys by tag and removes one", async () => {
    const { user, calls } = await openSettings({ quick: ["hardwareKey"] });
    expect(await screen.findByText("1 registered")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Manage" }));
    const dialog = await screen.findByRole("dialog", { name: "Security keys" });
    expect(within(dialog).getByText(/Security key · 01234567 · added/)).toBeTruthy();

    await user.click(within(dialog).getByRole("button", { name: "Remove security key 01234567" }));
    // Nothing is removed until it is confirmed.
    expect(within(dialog).getByText("Remove security key 01234567?")).toBeTruthy();
    expect(calls.some((c) => c.command === "remove_hardware_key")).toBe(false);
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(within(dialog).queryByText("Remove security key 01234567?")).toBeNull();
    await user.click(within(dialog).getByRole("button", { name: "Remove security key 01234567" }));
    await user.click(within(dialog).getByRole("button", { name: "Remove" }));
    expect(await within(dialog).findByText("None registered")).toBeTruthy();
    expect(calls.find((c) => c.command === "remove_hardware_key")?.request).toEqual({
      methodId: "0123456789abcdef0123456789abcdef",
    });
  });

  it("asks for the master password before raising the max age", async () => {
    const { user } = await openSettings();
    await user.selectOptions(await screen.findByLabelText("Ask for the master password at least every"), "30");
    const dialog = await screen.findByRole("dialog", { name: "Confirm with your master password" });
    await user.type(within(dialog).getByLabelText("Master password"), "mosaic");
    await user.click(within(dialog).getByRole("button", { name: "Confirm" }));
    await screen.findByText("Settings saved.");
  });

  it("hides the max-age choice where no quick method exists", async () => {
    await openSettings({ biometricPlatform: null });
    await screen.findByText("Biometric unlock isn't available on Linux.");
    expect(screen.queryByLabelText("Ask for the master password at least every")).toBeNull();
  });

  it("shows one \"Settings saved.\" toast for repeated saves", async () => {
    const { user } = await openSettings();
    const select = await screen.findByLabelText("Clear the clipboard after");
    await user.selectOptions(select, "60");
    await screen.findByText("Settings saved.");
    await user.selectOptions(select, "120");
    await waitFor(() => {
      expect(select).toHaveProperty("value", "120");
    });
    expect(screen.getAllByText("Settings saved.")).toHaveLength(1);
  });
});
