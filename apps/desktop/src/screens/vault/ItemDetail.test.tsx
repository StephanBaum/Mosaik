import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../../App.tsx";
import { SecretsProvider } from "../../app/SecretsContext.tsx";
import { ToastProvider } from "../../components/ToastProvider.tsx";
import { I18nProvider } from "../../i18n/I18nProvider.tsx";
import type { ItemDetail as Detail } from "../../ipc/contract/index.ts";
import { IpcProvider } from "../../ipc/IpcProvider.tsx";
import { createMockIpc } from "../../ipc/mock.ts";
import { DetailFields } from "./DetailFields.tsx";
import { groupCode } from "./TotpCode.tsx";

afterEach(() => {
  vi.useRealTimers();
});

async function openGitHub(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: /GitHub/ }));
  await screen.findByRole("article", { name: "GitHub" });
}

describe("item detail", () => {
  it("reveals a secret on request and hides it again on lock", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);

    expect(screen.queryByText("correct-horse-battery-staple")).toBeNull();
    expect(screen.getByRole("img", { name: /Hidden/ })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Reveal Password" }));
    expect(await screen.findByText("correct-horse-battery-staple")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Lock" }));
    await screen.findByRole("heading", { name: "Unlock Mosaic" });
    expect(document.body.textContent).not.toContain("correct-horse-battery-staple");
  });

  it("labels a single URL as Website, not Websites", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    const article = screen.getByRole("article", { name: "GitHub" });
    expect(within(article).getByText("Website")).toBeTruthy();
    expect(within(article).queryByText("Websites")).toBeNull();
  });

  it("hides a revealed secret after its timeout", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await user.click(screen.getByRole("button", { name: "Reveal Password" }));
    await screen.findByText("correct-horse-battery-staple");
    await vi.advanceTimersByTimeAsync(61_000);
    await waitFor(() => {
      expect(screen.queryByText("correct-horse-battery-staple")).toBeNull();
    });
  });

  it("copies and says when the clipboard clears", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    await user.click(screen.getByRole("button", { name: "Copy Password" }));
    expect(await screen.findByText("Copied. Clears in 30 seconds.")).toBeTruthy();
  });

  it("moves an item to Trash from the more-actions menu", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(screen.getByRole("menuitem", { name: "Move to Trash" }));
    await waitFor(() => {
      expect(screen.queryByRole("article", { name: "GitHub" })).toBeNull();
    });
  });

  it("asks before deleting forever and only purges on confirm", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(screen.getByRole("menuitem", { name: "Move to Trash" }));
    await waitFor(() => {
      expect(screen.queryByRole("article", { name: "GitHub" })).toBeNull();
    });

    await user.click(screen.getByRole("button", { name: /^Trash/ }));
    await user.click(await screen.findByRole("button", { name: /GitHub/ }));
    const article = await screen.findByRole("article", { name: "GitHub" });
    expect(screen.getByText(/days? left/)).toBeTruthy();

    await user.click(within(article).getByRole("button", { name: "Delete forever" }));
    const dialog = await screen.findByRole("dialog", { name: /Delete "GitHub" forever?/ });
    expect(within(dialog).getByText("You can't undo this.")).toBeTruthy();
    // Cancel keeps the item.
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("article", { name: "GitHub" })).toBeTruthy();

    await user.click(within(screen.getByRole("article", { name: "GitHub" })).getByRole("button", { name: "Delete forever" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Delete forever" }));
    await waitFor(() => {
      expect(screen.queryByRole("article", { name: "GitHub" })).toBeNull();
    });
    expect(await screen.findByText("Deleted for good.")).toBeTruthy();
  });

  it("shows the changed date in the footer", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    const article = screen.getByRole("article", { name: "GitHub" });
    expect(within(article).getByText(/^Changed /)).toBeTruthy();
  });
});

describe("groupCode", () => {
  it("splits six digits 3 + 3 and leaves other lengths alone", () => {
    expect(groupCode("123456")).toBe("123 456");
    expect(groupCode("12345678")).toBe("12345678");
  });
});

const ITEM = { vaultId: "v1", itemId: "i1" };

function detailOf(over: Partial<Detail> & { kind: Detail["summary"]["kind"] }): Detail {
  const { kind, ...rest } = over;
  return {
    summary: {
      ...ITEM,
      kind,
      title: "Item",
      subtitle: null,
      host: null,
      favorite: false,
      hasOtp: false,
      tags: [],
      modifiedAt: null,
      trashedAt: null,
      needsReview: false,
    },
    createdAt: null,
    text: [],
    secrets: [],
    urls: [],
    customFields: [],
    attachments: [],
    otp: null,
    historyCount: 0,
    review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
    sshKey: null,
    ...rest,
  };
}

function renderFields(detail: Detail) {
  return render(
    <I18nProvider locale="en">
      <IpcProvider ipc={createMockIpc({ start: "unlocked" })}>
        <ToastProvider>
          <SecretsProvider>
            <DetailFields detail={detail} item={ITEM} />
          </SecretsProvider>
        </ToastProvider>
      </IpcProvider>
    </I18nProvider>,
  );
}

describe("detail fields by kind", () => {
  it("groups the fields of an identity under their headings", () => {
    renderFields(
      detailOf({
        kind: "identity",
        text: [
          { field: "firstName", value: "Alex" },
          { field: "email", value: "alex@example.com" },
          { field: "city", value: "Berlin" },
          { field: "birthDate", value: "1990-01-02" },
        ],
        secrets: ["passportNumber"],
      }),
    );
    const headings = screen.getAllByRole("heading", { level: 3 }).map((each) => each.textContent);
    expect(headings).toEqual(["Name", "Contact", "Address", "ID documents"]);
    expect(screen.getByRole("button", { name: "Copy Email" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Copy Passport number" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Copy City" })).toBeNull();
  });

  it("marks a card whose expiry is over and copies the number but not the PIN", () => {
    renderFields(
      detailOf({
        kind: "creditCard",
        text: [{ field: "cardExpiry", value: "01/2020" }],
        secrets: ["cardNumber", "cardPin"],
      }),
    );
    expect(screen.getByText("Expired")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Copy PIN" })).toBeNull();
    expect(screen.getByRole("button", { name: "Copy Card number" })).toBeTruthy();
    expect(screen.queryByText(/4111/)).toBeNull();
  });

  it("shows no chip for a card that is still good", () => {
    renderFields(detailOf({ kind: "creditCard", text: [{ field: "cardExpiry", value: "04/2999" }] }));
    expect(screen.queryByText("Expired")).toBeNull();
  });

  it("shows the SSH public key, fingerprint and what the agent cannot use", () => {
    renderFields(
      detailOf({
        kind: "sshKey",
        text: [{ field: "sshComment", value: "deploy@example.com" }],
        secrets: ["password"],
        sshKey: {
          publicKey: "ssh-rsa AAAAB3 deploy",
          fingerprint: "SHA256:abc",
          algorithm: "ssh-rsa",
          encrypted: true,
          agentSupported: false,
        },
      }),
    );
    expect(screen.getByText("ssh-rsa AAAAB3 deploy")).toBeTruthy();
    expect(screen.getByText("SHA256:abc")).toBeTruthy();
    expect(screen.getByText("Protected by a passphrase")).toBeTruthy();
    expect(screen.getByText("The SSH agent can't use ssh-rsa keys yet. The key is stored safely.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Copy Public key" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reveal Passphrase" })).toBeTruthy();
    // The private key has a row that says so, and no way to see or copy it.
    expect(screen.getByText("Stored encrypted. It can't be shown or copied.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Private key/ })).toBeNull();
  });

  it("lists fields outside the layout under Other fields, and a one-time code field without a value", () => {
    renderFields(
      detailOf({
        kind: "login",
        text: [
          { field: "username", value: "alex" },
          { field: "company", value: "Acme" },
        ],
        secrets: ["password", "cardPin"],
        customFields: [
          { elementId: "c1", label: "Backup", kind: "otp", value: null },
          { elementId: "c2", label: "Door code", kind: "hidden", value: null },
        ],
      }),
    );
    expect(screen.getByRole("heading", { name: "Other fields" })).toBeTruthy();
    expect(screen.getByText("Company")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reveal Door code" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reveal PIN" })).toBeTruthy();
    const backup = screen.getByText("Backup").closest(".row");
    expect(backup?.textContent).toContain("One-time code");
    expect(within(backup as HTMLElement).queryByRole("button")).toBeNull();
  });

  it("lists every field of a kind without a layout", () => {
    renderFields(detailOf({ kind: "passkey", text: [{ field: "username", value: "alex" }] }));
    expect(screen.queryByRole("heading", { name: "Other fields" })).toBeNull();
    expect(screen.getByText("alex")).toBeTruthy();
  });
});
