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
    renderLocked({ quick: ["biometric"], latencyMs: 50 });
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
    expect((await screen.findAllByRole("alert")).some((a) => a.textContent === "Enter your security key's PIN.")).toBe(true);
    await user.type(pin, "0000");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect((await screen.findAllByRole("alert")).some((a) => /Wrong PIN/.test(a.textContent))).toBe(true);
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
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect((await screen.findAllByRole("alert")).some((a) => /Cancelled/.test(a.textContent))).toBe(true);
    expect(calls).toContain("hardware_key_cancel");
    expect(await screen.findByRole("button", { name: "Use security key" })).toBeTruthy();
  });
});
