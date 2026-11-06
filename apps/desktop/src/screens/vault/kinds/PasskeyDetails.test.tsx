import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../../../i18n/I18nProvider.tsx";
import type { PasskeySummary } from "../../../ipc/contract/index.ts";
import { PasskeyDetails } from "./PasskeyDetails.tsx";

const SUMMARY: PasskeySummary = {
  rpId: "example.com",
  userName: "alex@example.com",
  userDisplayName: "Alex Example",
  credentialId: "AAECAwQFBgcICQoLDA0ODw",
  algorithm: "es256",
  createdAt: Date.UTC(2026, 10, 5, 12),
  signCount: 0,
};

function show(summary: PasskeySummary, locale: "en" | "de" = "en") {
  return render(
    <I18nProvider locale={locale}>
      <div className="rows">
        <PasskeyDetails summary={summary} />
      </div>
    </I18nProvider>,
  );
}

describe("PasskeyDetails", () => {
  it("lists the site, the account, a short credential ID, the algorithm and the date", () => {
    show(SUMMARY);
    expect(screen.getByText("Website").nextSibling?.textContent).toBe("example.com");
    expect(screen.getByText("User name").nextSibling?.textContent).toBe("alex@example.com");
    expect(screen.getByText("Display name").nextSibling?.textContent).toBe("Alex Example");
    const id = screen.getByText("Credential ID").nextSibling as HTMLElement;
    expect(id.textContent).toBe("AAECAwQFBgcICQoL…");
    expect(id.getAttribute("title")).toBe("AAECAwQFBgcICQoLDA0ODw");
    expect(screen.getByText("Algorithm").nextSibling?.textContent).toBe("ES256");
    expect(screen.getByText("Created").nextSibling?.textContent).toMatch(/2026/);
  });

  it("gives the full credential ID to screen readers, not the shortened text", () => {
    show(SUMMARY);
    expect(screen.getByLabelText("AAECAwQFBgcICQoLDA0ODw").textContent).toBe("AAECAwQFBgcICQoL…");
  });

  it("says the key is end-to-end encrypted, syncs, is never shown and leaves only through the export", () => {
    show(SUMMARY);
    expect(
      screen.getByText(
        "This passkey is end-to-end encrypted and syncs with your vault. Its private key is never shown. You can only move it with Settings → Export passkeys.",
      ),
    ).toBeTruthy();
  });

  it("offers no way to see or copy the key or the credential", () => {
    show(SUMMARY);
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("leaves out empty names, shows a short ID whole, and names an unusable algorithm", () => {
    show({ ...SUMMARY, userName: "", userDisplayName: "", credentialId: "abcd", algorithm: "other" });
    expect(screen.queryByText("User name")).toBeNull();
    expect(screen.queryByText("Display name")).toBeNull();
    const id = screen.getByText("Credential ID").nextSibling as HTMLElement;
    expect(id.textContent).toBe("abcd");
    expect(screen.getByText("Algorithm").nextSibling?.textContent).toBe("Other (can't be used to sign in)");
  });

  it("is translated", () => {
    show(SUMMARY, "de");
    expect(screen.getByText("Website")).toBeTruthy();
    expect(screen.getByText("Benutzername")).toBeTruthy();
    expect(screen.getByText("Passkey-ID")).toBeTruthy();
    expect(screen.getByText("Algorithmus")).toBeTruthy();
    expect(screen.getByText(/nur über Einstellungen → Passkeys exportieren mitnehmen/)).toBeTruthy();
  });
});
