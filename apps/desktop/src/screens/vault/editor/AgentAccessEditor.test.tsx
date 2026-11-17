import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "../../../App.tsx";
import { SecretsProvider } from "../../../app/SecretsContext.tsx";
import { ToastProvider } from "../../../components/ToastProvider.tsx";
import { I18nProvider } from "../../../i18n/I18nProvider.tsx";
import type { MosaicIpc } from "../../../ipc/client.ts";
import type { CommandName, IpcError, ItemDetail } from "../../../ipc/contract/index.ts";
import { IpcProvider } from "../../../ipc/IpcProvider.tsx";
import { createMockIpc, type MockOptions } from "../../../ipc/mock.ts";
import { ItemEditor } from "../ItemEditor.tsx";

interface Sent {
  command: CommandName;
  request: unknown;
}

const SECRET = "sk-live-0123456789abcdef";

/** The editor on a new API key item, over the mock, recording what it sends. */
async function renderEditor(options: MockOptions, failAdd?: IpcError) {
  const base = createMockIpc({ start: "unlocked", ...options });
  const sent: Sent[] = [];
  const ipc: MosaicIpc = {
    subscribe: (listener) => base.subscribe(listener),
    call: (command, request) => {
      sent.push({ command, request });
      if (command === "add_item" && failAdd !== undefined) {
        return Promise.resolve({ ok: false, error: failAdd });
      }
      return base.call(command, request);
    },
  };
  const listed = await base.call("list_vaults", null);
  const vaults = listed.ok ? listed.value : [];
  render(
    <I18nProvider locale="en">
      <IpcProvider ipc={ipc}>
        <ToastProvider>
          <SecretsProvider>
            <ItemEditor
              vaults={vaults}
              vaultId={vaults[0]?.vaultId ?? null}
              editing={null}
              onSaved={vi.fn()}
              onCancel={vi.fn()}
            />
          </SecretsProvider>
        </ToastProvider>
      </IpcProvider>
    </I18nProvider>,
  );
  return { sent };
}

async function fillApiKey(user: ReturnType<typeof userEvent.setup>, secret = SECRET) {
  await user.selectOptions(screen.getByLabelText("Type"), "apiKey");
  await user.type(screen.getByLabelText("Title"), "Example API");
  await user.type(screen.getByLabelText("Secret"), secret);
}

async function group() {
  return await screen.findByRole("group", { name: "Agent access for Secret" });
}

const addItemRequests = (sent: Sent[]) => sent.filter((each) => each.command === "add_item");

describe("agent access in the item editor", () => {
  it("section is hidden while agent access is off", async () => {
    const user = userEvent.setup();
    await renderEditor({});
    await fillApiKey(user);
    // Let the status call settle: nothing appears.
    await waitFor(() => {
      expect(screen.getByLabelText("Secret")).toBeTruthy();
    });
    expect(screen.queryByRole("group", { name: /Agent access/ })).toBeNull();
    expect(screen.queryByText("Agents may use this value")).toBeNull();
  });

  it("switch on adds a binding change with canonical origins", async () => {
    const user = userEvent.setup();
    const { sent } = await renderEditor({ agent: "on" });
    await fillApiKey(user);
    const section = await group();
    const toggle = within(section).getByRole("switch", { name: "Agents may use this value" });
    expect((toggle as HTMLInputElement).checked).toBe(false);
    await user.click(toggle);
    const site = within(section).getByLabelText("Site 1");
    await user.type(site, "API.Example.com");
    expect(within(section).getByText("Stored as https://api.example.com")).toBeTruthy();
    await user.type(within(section).getByLabelText("Variable name (optional)"), "EXAMPLE_API_KEY");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => {
      expect(addItemRequests(sent)).toHaveLength(1);
    });
    const draft = (addItemRequests(sent)[0]?.request as { draft: { agentBindings?: unknown } }).draft;
    expect(draft.agentBindings).toEqual([
      {
        op: "add",
        field: { kind: "field", field: "apiKeySecret" },
        origins: ["https://api.example.com"],
        inject: { kind: "bearer", headerName: null, headerPrefix: null, basicUser: null, queryName: null },
        env: "EXAMPLE_API_KEY",
      },
    ]);
  });

  it("an off switch sends no binding at all", async () => {
    const user = userEvent.setup();
    const { sent } = await renderEditor({ agent: "on" });
    await fillApiKey(user);
    await group();
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => {
      expect(addItemRequests(sent)).toHaveLength(1);
    });
    expect((addItemRequests(sent)[0]?.request as { draft: object }).draft).not.toHaveProperty("agentBindings");
  });

  it("query kind shows the logging note", async () => {
    const user = userEvent.setup();
    await renderEditor({ agent: "on" });
    await fillApiKey(user);
    const section = await group();
    await user.click(within(section).getByRole("switch", { name: "Agents may use this value" }));
    expect(within(section).queryByText(/may write addresses into its logs/)).toBeNull();
    await user.selectOptions(within(section).getByLabelText("How it is sent"), "query");
    expect(within(section).getByText(/may write addresses into its logs/)).toBeTruthy();
    expect(within(section).getByLabelText("Parameter name")).toBeTruthy();
    await user.selectOptions(within(section).getByLabelText("How it is sent"), "header");
    expect(within(section).getByLabelText("Header name")).toBeTruthy();
    await user.selectOptions(within(section).getByLabelText("How it is sent"), "basic");
    expect(within(section).getByLabelText("User name")).toBeTruthy();
  });

  it("flags a look-alike address and an address that is not https", async () => {
    const user = userEvent.setup();
    await renderEditor({ agent: "on" });
    await fillApiKey(user);
    const section = await group();
    await user.click(within(section).getByRole("switch", { name: "Agents may use this value" }));
    const site = within(section).getByLabelText("Site 1");
    await user.type(site, "http://api.example.com");
    expect(within(section).getByText("Use an https address that names only the site.")).toBeTruthy();
    await user.clear(site);
    await user.type(site, "https://xn--pple-43d.example.com");
    expect(within(section).getByText(/characters that can pass for others/)).toBeTruthy();
  });

  it("server errors map to field messages", async () => {
    const user = userEvent.setup();
    // The mock itself refuses an address that is not https and a value that is too short.
    await renderEditor({ agent: "on" });
    await fillApiKey(user, "short");
    const section = await group();
    await user.click(within(section).getByRole("switch", { name: "Agents may use this value" }));
    await user.type(within(section).getByLabelText("Site 1"), "https://api.example.com");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText(/too short or has a line break/)).toBeTruthy();
    // The same message is not repeated at the bottom of the form.
    expect(screen.getAllByText(/too short or has a line break/)).toHaveLength(1);
  });

  it("maps the other agent codes the same way", async () => {
    const user = userEvent.setup();
    await renderEditor({ agent: "on" }, { code: "agentVaultShared" });
    await fillApiKey(user);
    const section = await group();
    await user.click(within(section).getByRole("switch", { name: "Agents may use this value" }));
    await user.type(within(section).getByLabelText("Site 1"), "https://api.example.com");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText(/Move the item to one of your own vaults first/)).toBeTruthy();
  });
});

describe("agent access in the item detail", () => {
  it("review banner shows the diff and confirm calls the command", async () => {
    const base = createMockIpc({ start: "unlocked", agent: "on" });
    const sent: Sent[] = [];
    let confirmed = false;
    const ipc: MosaicIpc = {
      subscribe: (listener) => base.subscribe(listener),
      call: async (command, request) => {
        sent.push({ command, request });
        const result = await base.call(command, request);
        if (command === "get_item" && !confirmed && result.ok) {
          const detail = result.value as ItemDetail;
          if (detail.summary.title === "GitHub") {
            const changed: ItemDetail = {
              ...detail,
              agentBindings: [
                {
                  elementId: "b1",
                  field: { kind: "field", field: "password" },
                  origins: ["https://api.example.com", "https://evil.example.net"],
                  inject: { kind: "header", headerName: "X-Api-Key", headerPrefix: "", basicUser: null, queryName: null },
                  env: null,
                  confirmed: false,
                  lastConfirmed: { origins: ["https://api.example.com", "https://old.example.com"], inject: "bearer" },
                  usable: false,
                },
              ],
            };
            return { ok: true, value: changed };
          }
        }
        if (command === "agent_binding_confirm") {
          confirmed = true;
          const detail = await base.call("get_item", request as { vaultId: string; itemId: string });
          return detail;
        }
        return result;
      },
    };
    const user = userEvent.setup();
    render(<App ipc={ipc} languages={["en"]} />);
    await user.click(await screen.findByRole("button", { name: /GitHub/ }));
    const article = await screen.findByRole("article", { name: "GitHub" });
    const review = await within(article).findByRole("group", {
      name: "Changed on another device, or set up elsewhere: review before agents can use it",
    });
    expect(within(review).getByText("Added: https://evil.example.net")).toBeTruthy();
    expect(within(review).getByText("Removed: https://old.example.com")).toBeTruthy();
    expect(within(review).getByText(/How it is sent changed from Bearer token to Header/)).toBeTruthy();
    // It is not usable until the user confirms it here.
    expect(within(article).queryByText("Agents may use this")).toBeNull();
    await user.click(within(review).getByRole("button", { name: "Confirm for this device" }));
    await waitFor(() => {
      expect(sent.filter((each) => each.command === "agent_binding_confirm")).toHaveLength(1);
    });
    expect(sent.find((each) => each.command === "agent_binding_confirm")?.request).toMatchObject({
      elementId: "b1",
    });
  });

  it("shows a confirmed binding without a banner", async () => {
    const user = userEvent.setup();
    const ipc = createMockIpc({ start: "unlocked", agent: "on" });
    const items = await ipc.call("list_items", { vaultId: null, trash: "exclude", favoritesOnly: false, query: null, limit: null });
    const github = items.ok ? items.value.find((item) => item.title === "GitHub") : undefined;
    expect(github).toBeDefined();
    await ipc.call("update_item", {
      vaultId: github?.vaultId ?? "",
      itemId: github?.itemId ?? "",
      draft: {
        text: [],
        secrets: [],
        urls: [],
        customFields: [],
        tags: null,
        otp: null,
        sshKey: null,
        agentBindings: [
          {
            op: "add",
            field: { kind: "field", field: "password" },
            origins: ["api.example.com"],
            inject: { kind: "bearer", headerName: null, headerPrefix: null, basicUser: null, queryName: null },
            env: null,
          },
        ],
      },
    });
    render(<App ipc={ipc} languages={["en"]} />);
    await user.click(await screen.findByRole("button", { name: /GitHub/ }));
    const article = await screen.findByRole("article", { name: "GitHub" });
    expect(await within(article).findByText("https://api.example.com")).toBeTruthy();
    expect(within(article).getByText("Agents may use this")).toBeTruthy();
    expect(within(article).queryByRole("button", { name: "Confirm for this device" })).toBeNull();
  });
});
