/**
 * Every component that renders a secret value must mark it, so the lock snapshot paints the
 * mask and never the value (UX_GUIDELINES section 8, ADR-0032). These render the real
 * components with the mock IPC, show the value, and run the snapshot collector over them.
 */
import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";
import { SecretsProvider } from "../app/SecretsContext.tsx";
import { SecretField } from "../components/SecretField.tsx";
import { ToastProvider } from "../components/ToastProvider.tsx";
import { I18nProvider } from "../i18n/I18nProvider.tsx";
import { IpcProvider } from "../ipc/IpcProvider.tsx";
import { createMockIpc } from "../ipc/mock.ts";
import { GeneratorPanel } from "../screens/GeneratorPanel.tsx";
import { TotpCode } from "../screens/vault/TotpCode.tsx";
import { collect } from "./collect.ts";
import type { Measure } from "./ops.ts";

const measure: Measure = {
  rect: () => ({ x: 0, y: 0, w: 800, h: 600 }),
  textRects: () => [{ x: 0, y: 0, w: 80, h: 14 }],
};

const MASK = "•".repeat(16);

function pictureOf(container: HTMLElement): string {
  // Live canvases are ops of their own; they are not text, so they stay out of the string.
  return JSON.stringify(collect(container, measure, { groups: false }), (_key, value: unknown) =>
    value instanceof Element ? "[element]" : value,
  );
}

function renderWithApp(ui: ReactNode) {
  return render(
    <I18nProvider locale="en">
      <IpcProvider ipc={createMockIpc({ start: "unlocked" })}>
        <ToastProvider>
          <SecretsProvider>{ui}</SecretsProvider>
        </ToastProvider>
      </IpcProvider>
    </I18nProvider>,
  );
}

describe("secrets stay out of the lock snapshot", () => {
  it("SecretField: the hidden mask and the revealed value both carry data-secret", async () => {
    const secret = "correct-horse-battery-staple";
    const { container } = renderWithApp(
      <SecretField
        label="Password"
        reveal={() => Promise.resolve({ ok: true, value: { value: secret, hideAfterSeconds: 30 } })}
      />,
    );
    const hidden = container.querySelector("[data-secret]");
    expect(hidden?.getAttribute("data-mask")).toBe(MASK);
    expect(pictureOf(container)).toContain(MASK);

    fireEvent.click(screen.getByRole("button", { name: "Reveal Password" }));
    const shown = await screen.findByText(secret);
    // Sanity: the value is in the DOM, and its element is marked.
    expect(container.textContent).toContain(secret);
    expect(shown.hasAttribute("data-secret")).toBe(true);
    expect(shown.getAttribute("data-mask")).toBe(MASK);

    const picture = pictureOf(container);
    expect(picture).not.toContain(secret);
    expect(picture).toContain(MASK);
    expect(picture).toContain("Password");
  });

  it("TotpCode: the live code carries data-secret", async () => {
    const { container } = renderWithApp(
      <TotpCode
        source={{
          kind: "item",
          item: { vaultId: "019a2b3c-4d5e-7010-8000-000000000001", itemId: "019a2b3c-4d5e-7020-8000-000000000001" },
        }}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Show code" }));
    const code = await screen.findByText(/^\d{3} \d{3}$/);
    expect(code.hasAttribute("data-secret")).toBe(true);
    expect(code.getAttribute("data-mask")).toBe("••• •••");
    expect(pictureOf(container)).not.toContain(code.textContent);
  });

  it("GeneratorPanel: the generated password carries data-secret", async () => {
    const { container } = renderWithApp(<GeneratorPanel />);
    const generated = await waitForOutput(container);
    expect(generated.hasAttribute("data-secret")).toBe(true);
    const value = generated.textContent;
    expect(value.length).toBeGreaterThan(8);
    expect(pictureOf(container)).not.toContain(value);
  });
});

async function waitForOutput(container: HTMLElement): Promise<HTMLElement> {
  for (let i = 0; i < 100; i++) {
    const output = container.querySelector<HTMLElement>("output.generated");
    if (output !== null && output.textContent.length > 0) {
      return output;
    }
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  throw new Error("the generator never produced a value");
}
