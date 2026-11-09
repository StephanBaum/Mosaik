import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import { createMockIpc, type MockOptions } from "../ipc/mock.ts";
import { PlatformContext, type Platform } from "../platform/platform.ts";

function renderLocked(options: MockOptions, platform: Platform = "windows") {
  const calls: string[] = [];
  const mock = createMockIpc({ start: "locked", ...options });
  const ipc: MosaicIpc = {
    subscribe: (listener) => mock.subscribe(listener),
    call: (command, request) => {
      calls.push(command);
      return mock.call(command, request);
    },
  };
  render(
    <PlatformContext value={platform}>
      <App ipc={ipc} languages={["en"]} />
    </PlatformContext>,
  );
  return { calls };
}

describe("unlock screen, quick unlock", () => {
  it("offers no quick buttons when nothing is enrolled", async () => {
    renderLocked({});
    await screen.findByLabelText("Master password");
    expect(screen.queryByRole("button", { name: /Windows Hello|Touch ID|security key/i })).toBeNull();
  });

  it("names the biometric by platform and shows the waiting line until the vault opens", async () => {
    const user = userEvent.setup();
    // The waiting line only shows while the call runs; a loaded machine needs a wide window to see it.
    renderLocked({ quick: ["biometric"], latencyMs: 600 });
    const button = await screen.findByRole("button", { name: "Use Windows Hello" });
    await user.click(button);
    await screen.findByText("Waiting for Windows Hello…");
    expect(await screen.findByText("GitHub")).toBeTruthy();
  });

  it("uses the Touch ID wording on macOS", async () => {
    renderLocked({ quick: ["biometric"] }, "mac");
    expect(await screen.findByRole("button", { name: "Use Touch ID" })).toBeTruthy();
  });

  it("asks for the PIN, clears it after a wrong one and then unlocks", async () => {
    const user = userEvent.setup();
    renderLocked({ quick: ["hardwareKey"] });
    await user.click(await screen.findByRole("button", { name: "Use security key" }));

    const pin = await screen.findByLabelText("Security key PIN");
    // Asking for the PIN is a prompt, not an error: nothing is marked invalid yet.
    expect(pin.getAttribute("aria-invalid")).toBeNull();
    await user.type(pin, "0000");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    // The wrong PIN is reported at the PIN field, not in the password form.
    const field = await screen.findByLabelText("Security key PIN");
    await waitFor(() => {
      expect(field.getAttribute("aria-invalid")).toBe("true");
    });
    const described = field.getAttribute("aria-describedby") ?? "";
    expect(document.getElementById(described)?.textContent).toMatch(/Wrong PIN/);
    expect(screen.getByLabelText("Master password").closest("form")?.textContent).not.toMatch(/Wrong PIN/);
    expect(screen.getByLabelText<HTMLInputElement>("Security key PIN").value).toBe("");

    await user.type(screen.getByLabelText("Security key PIN"), "1234");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(await screen.findByText("GitHub")).toBeTruthy();
  });

  it("drops the buttons, focuses the password and explains when quick unlock expired", async () => {
    const user = userEvent.setup();
    renderLocked({ quick: ["biometric"], quickFail: "quickUnlockExpired" });
    await user.click(await screen.findByRole("button", { name: "Use Windows Hello" }));

    expect((await screen.findAllByRole("alert")).some((a) => /Quick unlock expired/.test(a.textContent))).toBe(true);
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "Use Windows Hello" })).toBeNull();
    });
    expect(document.activeElement).toBe(screen.getByLabelText("Master password"));
  });

  it("cancels a pending security-key touch", async () => {
    const user = userEvent.setup();
    const { calls } = renderLocked({ quick: ["hardwareKey"], touchMs: 60_000 });
    await user.click(await screen.findByRole("button", { name: "Use security key" }));
    await user.type(await screen.findByLabelText("Security key PIN"), "1234");
    await user.click(screen.getByRole("button", { name: "Continue" }));

    await screen.findByText("Insert your key and touch it when it blinks.");
    // Cancel has focus, and the password form cannot be sent while the key is waited for.
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Unlock" }).disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    const again = await screen.findByRole("button", { name: "Use security key" });
    // A deliberate cancel shows nothing and focus returns to the method used.
    expect(screen.queryByText(/Cancelled/)).toBeNull();
    expect(calls).toContain("hardware_key_cancel");
    await waitFor(() => {
      expect(document.activeElement).toBe(again);
    });
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Unlock" }).disabled).toBe(false);
  });

  it("shows a security-key or biometric error next to the buttons, not in the password form", async () => {
    const user = userEvent.setup();
    renderLocked({ quick: ["biometric"], quickFail: "busy" });
    const button = await screen.findByRole("button", { name: "Use Windows Hello" });
    await user.click(button);
    const message = await screen.findByText(/still finishing the last step/);
    expect(screen.getByLabelText("Master password").closest("form")?.contains(message)).toBe(false);
    expect(button.getAttribute("aria-describedby")).toBe(message.id);
    // Using the password form clears it.
    await user.type(screen.getByLabelText("Master password"), "wrong");
    await user.click(screen.getByRole("button", { name: "Unlock" }));
    await waitFor(() => {
      expect(screen.queryByText(/still finishing the last step/)).toBeNull();
    });
  });
});
