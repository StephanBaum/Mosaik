import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import type { CommandName } from "../ipc/contract/index.ts";
import { createMockIpc, type MockOptions } from "../ipc/mock.ts";
import { configFor, pairCommand, shellPath } from "./AgentSetup.tsx";

interface Sent {
  command: CommandName;
  request: unknown;
}

/** The mock, recording every command that goes through it. */
function recording(options: MockOptions): { ipc: MosaicIpc; sent: Sent[] } {
  const base = createMockIpc(options);
  const sent: Sent[] = [];
  const ipc: MosaicIpc = {
    subscribe: (listener) => base.subscribe(listener),
    call: (command, request) => {
      sent.push({ command, request });
      return base.call(command, request);
    },
  };
  return { ipc, sent };
}

async function openAgentSection(options: MockOptions, locale: "en" | "de" = "en") {
  const { ipc, sent } = recording(options);
  const user = userEvent.setup();
  render(<App ipc={ipc} languages={[locale]} />);
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: locale === "en" ? "Settings" : "Einstellungen" }));
  const heading = await screen.findByRole("heading", { name: locale === "en" ? "AI agents" : "KI-Agenten" });
  return { user, sent, section: heading.closest("section") as HTMLElement };
}

const callsOf = (sent: Sent[], command: CommandName) => sent.filter((each) => each.command === command);

describe("agent settings", () => {
  it("enable switch calls set_enabled and then shows the setup help", async () => {
    const { user, sent, section } = await openAgentSection({ start: "unlocked" });
    const toggle = within(section).getByRole("switch", { name: "Allow AI agents to use secrets" });
    expect((toggle as HTMLInputElement).checked).toBe(false);
    expect(within(section).queryByText("Connect an agent")).toBeNull();
    await user.click(toggle);
    await waitFor(() => {
      expect(callsOf(sent, "agent_access_set_enabled")).toEqual([
        { command: "agent_access_set_enabled", request: { enabled: true } },
      ]);
    });
    expect(await within(section).findByText("Connect an agent")).toBeTruthy();
    expect(within(section).getByText(/No agent is connected yet/)).toBeTruthy();
    await user.click(within(section).getByRole("switch", { name: "Allow AI agents to use secrets" }));
    await waitFor(() => {
      expect(within(section).queryByText("Connect an agent")).toBeNull();
    });
  });

  it("remove and revoke call their commands", async () => {
    const { user, sent, section } = await openAgentSection({ start: "unlocked", agent: "on" });
    expect(await within(section).findByText("Claude Code may use Example API")).toBeTruthy();
    await user.click(
      within(section).getByRole("button", { name: "Revoke the permission of Claude Code for Example API" }),
    );
    await waitFor(() => {
      expect(callsOf(sent, "agent_grant_revoke")).toEqual([
        { command: "agent_grant_revoke", request: { grantId: 1 } },
      ]);
    });
    await waitFor(() => {
      expect(within(section).getByText(/No agent has permission/)).toBeTruthy();
    });

    // Remove asks first, and Cancel keeps the agent.
    await user.click(within(section).getByRole("button", { name: "Remove Claude Code" }));
    expect(within(section).getByText("Remove Claude Code?")).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: "Cancel" }));
    expect(callsOf(sent, "agent_pairing_remove")).toEqual([]);
    await user.click(within(section).getByRole("button", { name: "Remove Claude Code" }));
    await user.click(within(section).getByRole("button", { name: "Remove" }));
    await waitFor(() => {
      expect(callsOf(sent, "agent_pairing_remove")).toEqual([
        {
          command: "agent_pairing_remove",
          request: { pairingId: "0123456789abcdef0123456789abcdef" },
        },
      ]);
    });
    expect(await within(section).findByText(/No agent is connected yet/)).toBeTruthy();
  });

  it("stop all asks once and calls stop_all", async () => {
    const { user, sent, section } = await openAgentSection({ start: "unlocked", agent: "on" });
    await user.click(await within(section).findByRole("button", { name: "Stop all agent access now" }));
    expect(callsOf(sent, "agent_stop_all")).toEqual([]);
    expect(within(section).getByText("Stop all agent access?")).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: "Stop all" }));
    await waitFor(() => {
      expect(callsOf(sent, "agent_stop_all")).toHaveLength(1);
    });
    await waitFor(() => {
      expect(within(section).getByText(/No agent has permission/)).toBeTruthy();
    });
  });

  it("setup snippet contains the CLI path and no secret", async () => {
    const { user, section } = await openAgentSection({ start: "unlocked", agent: "on" });
    expect(
      await within(section).findByText('/usr/local/bin/mosaic agent pair --name "Claude Code"'),
    ).toBeTruthy();
    const snippet = within(section).getByText(/claude mcp add mosaic/);
    expect(snippet.textContent).toBe(
      'claude mcp add mosaic -- /usr/local/bin/mosaic agent mcp --client "Claude Code"',
    );
    await user.selectOptions(within(section).getByLabelText("Program"), "claudeDesktop");
    const json = within(section).getByText(/mcpServers/);
    expect(JSON.parse(json.textContent)).toEqual({
      mcpServers: {
        mosaic: {
          command: "/usr/local/bin/mosaic",
          args: ["agent", "mcp", "--client", "Claude Desktop"],
        },
      },
    });
    expect(within(section).getByText(/Do not keep a command line session open/)).toBeTruthy();
  });

  it("activity marks refusals and says why in plain words", async () => {
    const { section } = await openAgentSection({ start: "unlocked", agent: "on" });
    const refused = (await within(section).findByText("Claude Code was refused for Example API")).closest("li");
    expect(refused?.className).toContain("agent-activity__entry--refused");
    expect(within(refused as HTMLElement).getByText(/refused: the site is not on the list/)).toBeTruthy();
    const used = within(section).getByText("Claude Code used Example API").closest("li");
    expect(used?.className).not.toContain("--refused");
    expect(within(used as HTMLElement).getByText(/GET https:\/\/api\.example\.com\/v1\/invoices/)).toBeTruthy();
  });

  it("speaks German with du", async () => {
    const { section } = await openAgentSection({ start: "unlocked", agent: "on" }, "de");
    expect(within(section).getByText(/Lass Programme wie Claude Code/)).toBeTruthy();
    expect(await within(section).findByText("Einen Agenten verbinden")).toBeTruthy();
  });
});

describe("setup text", () => {
  it("quotes a path with spaces and leaves a plain one alone", () => {
    expect(shellPath("/usr/bin/mosaic")).toBe("/usr/bin/mosaic");
    expect(shellPath("C:\\Program Files\\Mosaic\\mosaic.exe")).toBe('"C:\\Program Files\\Mosaic\\mosaic.exe"');
    expect(shellPath('a"b c')).toBe('"ab c"');
    expect(pairCommand(null, "cursor")).toBe('mosaic agent pair --name "Cursor"');
  });

  it("writes the configuration of each program with the same client name as the pairing", () => {
    const cli = "C:\\Program Files\\Mosaic\\mosaic.exe";
    expect(configFor(cli, "codex")).toBe(
      [
        "[mcp_servers.mosaic]",
        'command = "C:\\\\Program Files\\\\Mosaic\\\\mosaic.exe"',
        'args = ["agent","mcp","--client","Codex"]',
      ].join("\n"),
    );
    expect(JSON.parse(configFor(cli, "cursor"))).toEqual({
      mcpServers: { mosaic: { command: cli, args: ["agent", "mcp", "--client", "Cursor"] } },
    });
  });
});
