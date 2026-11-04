import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SecretsProvider } from "../../../app/SecretsContext.tsx";
import { ToastProvider } from "../../../components/ToastProvider.tsx";
import { I18nProvider } from "../../../i18n/I18nProvider.tsx";
import type { MosaicIpc } from "../../../ipc/client.ts";
import type { VaultSummary } from "../../../ipc/contract/index.ts";
import { IpcProvider } from "../../../ipc/IpcProvider.tsx";
import { createMockIpc, type MockOptions } from "../../../ipc/mock.ts";
import { PlatformContext } from "../../../platform/platform.ts";
import type { OtpSetupResult, OtpStart } from "./OtpSection.tsx";
import { OtpSetupDialog } from "./OtpSetupDialog.tsx";

const PERSONAL: VaultSummary = {
  vaultId: "v1",
  name: "Personal",
  itemCount: 0,
  available: true,
  protection: "accountKey",
  openUntil: null,
};
const CODES: VaultSummary = {
  vaultId: "v2",
  name: "Codes",
  itemCount: 0,
  available: true,
  protection: "hardwareKey",
  openUntil: null,
};

/** The mock, recording every command the dialog sends. */
function spied(options: MockOptions): { ipc: MosaicIpc; calls: string[] } {
  const inner = createMockIpc({ start: "unlocked", ...options });
  const calls: string[] = [];
  return {
    calls,
    ipc: {
      subscribe: (listener) => inner.subscribe(listener),
      call: (command, request) => {
        calls.push(command);
        return inner.call(command, request);
      },
    },
  };
}

function renderDialog(
  start: OtpStart,
  options: MockOptions = {},
  vaults: readonly VaultSummary[] = [PERSONAL],
) {
  const { ipc, calls } = spied(options);
  const onDone = vi.fn<(result: OtpSetupResult) => void>();
  const onClose = vi.fn();
  render(
    <PlatformContext value="windows">
      <I18nProvider locale="en">
        <IpcProvider ipc={ipc}>
          <ToastProvider>
            <SecretsProvider>
              <OtpSetupDialog
                start={start}
                prefill={{ issuer: "Acme", account: "alex" }}
                vaults={vaults}
                itemKind="login"
                screenScan
                onDone={onDone}
                onClose={onClose}
              />
            </SecretsProvider>
          </ToastProvider>
        </IpcProvider>
      </I18nProvider>
    </PlatformContext>,
  );
  return { calls, onDone, onClose };
}

const scan = (source: "screen" | "clipboard" | "file"): OtpStart => ({ kind: "scan", source });

describe("one-time code setup dialog", () => {
  it("scans the screen and shows what it found with the live code", async () => {
    const { calls } = renderDialog(scan("screen"));
    expect(await screen.findByRole("heading", { name: "Is this the right code?" })).toBeTruthy();
    expect((screen.getByLabelText("Service") as HTMLInputElement).value).toBe("GitHub");
    expect((screen.getByLabelText("Account") as HTMLInputElement).value).toBe("alex@example.com");
    expect(screen.getByText("6 digits, new code every 30 seconds")).toBeTruthy();
    expect(await screen.findByText("492 039")).toBeTruthy();
    expect(calls.filter((call) => call === "otp_scan")).toHaveLength(1);
  });

  it("hands the confirmed setup over with the edited labels", async () => {
    const user = userEvent.setup();
    const { onDone } = renderDialog(scan("screen"));
    await screen.findByRole("heading", { name: "Is this the right code?" });
    await user.clear(screen.getByLabelText("Service"));
    await user.type(screen.getByLabelText("Service"), "Work GitHub");
    await user.click(screen.getByRole("button", { name: "Use this code" }));
    expect(onDone).toHaveBeenCalledTimes(1);
    const result = onDone.mock.calls[0]?.[0];
    expect(result?.issuer).toBe("Work GitHub");
    expect(result?.accountName).toBe("alex@example.com");
    expect(result?.target).toEqual({ kind: "item" });
    expect(result?.pending.pendingId).toMatch(/^otp-/);
  });

  it("says what went wrong and offers the sources again", async () => {
    renderDialog(scan("screen"), { otpScan: "otpQrNotFound" });
    expect(
      await screen.findByText("No QR code found. Make sure the whole code is visible, then try again."),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Paste screenshot" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Can't scan? Enter the key" })).toBeTruthy();
  });

  it("closes silently when the file dialog is dismissed", async () => {
    const { onClose } = renderDialog(scan("file"), { otpScan: "cancel" });
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1);
    });
    expect(screen.queryByRole("alert")?.textContent ?? "").toBe("");
  });

  it("takes an otpauth link typed by hand", async () => {
    const user = userEvent.setup();
    renderDialog({ kind: "manual" });
    await user.type(
      screen.getByLabelText("Setup key or otpauth:// link"),
      "otpauth://totp/Acme:alex?secret=JBSWY3DPEHPK3PXP&issuer=Acme",
    );
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(await screen.findByRole("heading", { name: "Is this the right code?" })).toBeTruthy();
    expect((screen.getByLabelText("Service") as HTMLInputElement).value).toBe("Acme");
  });

  it("prefills service and account from the item for a typed key", async () => {
    const user = userEvent.setup();
    renderDialog({ kind: "manual" });
    expect((screen.getByLabelText("Service") as HTMLInputElement).value).toBe("Acme");
    expect((screen.getByLabelText("Account") as HTMLInputElement).value).toBe("alex");
    await user.type(screen.getByLabelText("Setup key or otpauth:// link"), "JBSW Y3DP EHPK 3PXP");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await screen.findByRole("heading", { name: "Is this the right code?" });
    expect((screen.getByLabelText("Service") as HTMLInputElement).value).toBe("Acme");
    expect((screen.getByLabelText("Account") as HTMLInputElement).value).toBe("alex");
  });

  it("rejects a key that is not base32", async () => {
    const user = userEvent.setup();
    renderDialog({ kind: "manual" });
    await user.type(screen.getByLabelText("Setup key or otpauth:// link"), "0189");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(await screen.findByText(/This key isn't valid/)).toBeTruthy();
    expect((screen.getByLabelText("Setup key or otpauth:// link") as HTMLInputElement).value).toBe("");
  });

  it("reads a pasted screenshot on Ctrl+V outside a text field and says the clipboard was cleared", async () => {
    const user = userEvent.setup();
    const { calls } = renderDialog(scan("file"));
    await screen.findByRole("heading", { name: "Is this the right code?" });
    // A browser focuses the first control of a modal dialog; jsdom does not.
    screen.getByRole("button", { name: "Scan again" }).focus();
    await user.keyboard("{Control>}v{/Control}");
    expect(
      await screen.findByText(/If clipboard history or the Snipping Tool kept a copy/),
    ).toBeTruthy();
    expect(calls.filter((call) => call === "otp_scan")).toHaveLength(2);
    expect(calls).toContain("otp_pending_discard");
  });

  it("leaves Ctrl+V alone inside a text field", async () => {
    const user = userEvent.setup();
    const { calls } = renderDialog({ kind: "manual" });
    await user.click(screen.getByLabelText("Setup key or otpauth:// link"));
    await user.keyboard("{Control>}v{/Control}");
    expect(calls).not.toContain("otp_scan");
  });

  it("forgets the pending setup when it is closed", async () => {
    const { calls, onClose } = renderDialog(scan("screen"));
    await screen.findByRole("heading", { name: "Is this the right code?" });
    fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(calls).toContain("otp_pending_discard");
  });

  it("warns about a short secret", async () => {
    const user = userEvent.setup();
    renderDialog({ kind: "manual" });
    await user.type(screen.getByLabelText("Setup key or otpauth:// link"), "JBSWY3DP");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(await screen.findByText(/This secret is unusually short/)).toBeTruthy();
  });

  it("shows a note instead of a code for a counter-based setup", async () => {
    const user = userEvent.setup();
    renderDialog({ kind: "manual" });
    await user.type(
      screen.getByLabelText("Setup key or otpauth:// link"),
      "otpauth://hotp/Acme:alex?secret=JBSWY3DPEHPK3PXP&counter=3",
    );
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(await screen.findByText(/Counter-based codes appear after you save/)).toBeTruthy();
    expect(screen.queryByText("Current code")).toBeNull();
  });

  it("lists a hardware vault as a place for the code and disables a closed one", async () => {
    renderDialog(scan("screen"), {}, [PERSONAL, CODES]);
    await screen.findByRole("heading", { name: "Is this the right code?" });
    const option = screen.getByRole("radio", { name: /In Codes \(needs your security key\)/ });
    expect((option as HTMLInputElement).disabled).toBe(false);
  });

  it("does not offer hardware storage without a hardware vault", async () => {
    renderDialog(scan("screen"));
    await screen.findByRole("heading", { name: "Is this the right code?" });
    expect(screen.queryByRole("radio")).toBeNull();
  });

  it("disables a hardware vault that is closed", async () => {
    renderDialog(scan("screen"), {}, [PERSONAL, { ...CODES, available: false, name: null }]);
    await screen.findByRole("heading", { name: "Is this the right code?" });
    const option = screen.getByRole("radio", { name: /security key/ });
    expect((option as HTMLInputElement).disabled).toBe(true);
  });
});
