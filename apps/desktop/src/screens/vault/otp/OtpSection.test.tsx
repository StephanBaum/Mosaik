import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { SecretsProvider } from "../../../app/SecretsContext.tsx";
import { ToastProvider } from "../../../components/ToastProvider.tsx";
import { I18nProvider } from "../../../i18n/I18nProvider.tsx";
import type { ItemKind, OtpSummary } from "../../../ipc/contract/index.ts";
import { IpcProvider } from "../../../ipc/IpcProvider.tsx";
import { createMockIpc, type MockOptions } from "../../../ipc/mock.ts";
import { OTP_UNCHANGED, OtpSection, type OtpFormValue } from "./OtpSection.tsx";

const EXISTING: OtpSummary = {
  scheme: "totp",
  issuer: "GitHub",
  digits: 6,
  period: 30,
  hardwareProtected: false,
};

function Host({ kind, existing }: { kind: ItemKind; existing: OtpSummary | null }) {
  const [value, setValue] = useState<OtpFormValue>(OTP_UNCHANGED);
  return (
    <OtpSection
      kind={kind}
      existing={existing}
      value={value}
      onChange={setValue}
      vaults={[]}
      prefill={{ issuer: "", account: "" }}
    />
  );
}

function renderSection(kind: ItemKind, existing: OtpSummary | null, options: MockOptions = {}) {
  return render(
    <I18nProvider locale="en">
      <IpcProvider ipc={createMockIpc({ start: "unlocked", ...options })}>
        <ToastProvider>
          <SecretsProvider>
            <Host kind={kind} existing={existing} />
          </SecretsProvider>
        </ToastProvider>
      </IpcProvider>
    </I18nProvider>,
  );
}

describe("one-time code section", () => {
  it("offers every source for a new login", async () => {
    renderSection("login", null);
    expect(await screen.findByRole("button", { name: "Scan screen" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Paste screenshot" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Choose image…" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Can't scan? Enter the key" })).toBeTruthy();
    expect(screen.getByText("Add the code from the site's two-factor setup page.")).toBeTruthy();
  });

  it("hides Scan screen where the system cannot scan it", async () => {
    renderSection("login", null, { screenScan: false });
    await screen.findByRole("button", { name: "Paste screenshot" });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(screen.queryByRole("button", { name: "Scan screen" })).toBeNull();
    expect(screen.getByRole("button", { name: "Choose image…" })).toBeTruthy();
  });

  it("has no section for an item that cannot hold a code", async () => {
    const { container } = renderSection("secureNote", null);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(container.querySelector(".otp-section")).toBeNull();
    expect(screen.queryByText("One-time code (2FA)")).toBeNull();
  });

  it("shows the code an item has and removes it on request", async () => {
    const user = userEvent.setup();
    renderSection("login", EXISTING);
    expect(await screen.findByText("GitHub")).toBeTruthy();
    expect(screen.getByText("6 digits, new code every 30 seconds")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Replace" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Remove one-time code" }));
    expect(screen.getByText("The one-time code will be removed when you save.")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Keep it" }));
    expect(screen.getByText("GitHub")).toBeTruthy();
  });

  it("offers the sources again on Replace", async () => {
    const user = userEvent.setup();
    renderSection("login", EXISTING);
    await user.click(await screen.findByRole("button", { name: "Replace" }));
    expect(screen.getByRole("button", { name: "Paste screenshot" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Keep it" }));
    expect(screen.getByRole("button", { name: "Remove one-time code" })).toBeTruthy();
  });
});
