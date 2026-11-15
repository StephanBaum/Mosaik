/**
 * The setup help of the agent settings: the command that connects a program, and the
 * configuration to paste into it (Claude Code, Claude Desktop, Cursor, Codex). Built from the
 * path of the `mosaic` program Rust reports; nothing secret is in it. Also the advice not to
 * keep a command line session open while an agent can run commands (ADR-0159 Q10).
 */
import { useState } from "react";
import { Button } from "../components/Button.tsx";
import { Callout } from "../components/Callout.tsx";
import { Select } from "../components/Select.tsx";
import { useToast } from "../components/ToastProvider.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";

export const AGENT_CLIENTS = ["claudeCode", "claudeDesktop", "cursor", "codex"] as const;
export type AgentClient = (typeof AGENT_CLIENTS)[number];

/** The name the program connects under: the same in `pair --name` and `mcp --client`. */
const CLIENT_NAME: Record<AgentClient, string> = {
  claudeCode: "Claude Code",
  claudeDesktop: "Claude Desktop",
  cursor: "Cursor",
  codex: "Codex",
};

/** A path for a shell: quoted when it holds anything but plain path characters. */
export function shellPath(path: string): string {
  return /^[\w./\\:-]+$/.test(path) ? path : `"${path.replaceAll('"', "")}"`;
}

/** `mosaic agent pair --name "Claude Code"`, with the path of the program when it is known. */
export function pairCommand(cliPath: string | null, client: AgentClient): string {
  return `${shellPath(cliPath ?? "mosaic")} agent pair --name "${CLIENT_NAME[client]}"`;
}

function json(command: string, client: AgentClient): string {
  return JSON.stringify(
    { mcpServers: { mosaic: { command, args: ["agent", "mcp", "--client", CLIENT_NAME[client]] } } },
    null,
    2,
  );
}

/** The configuration of one program. */
export function configFor(cliPath: string | null, client: AgentClient): string {
  const command = cliPath ?? "mosaic";
  switch (client) {
    case "claudeCode":
      return `claude mcp add mosaic -- ${shellPath(command)} agent mcp --client "${CLIENT_NAME[client]}"`;
    case "codex":
      return [
        "[mcp_servers.mosaic]",
        `command = ${JSON.stringify(command)}`,
        `args = ${JSON.stringify(["agent", "mcp", "--client", CLIENT_NAME[client]])}`,
      ].join("\n");
    default:
      return json(command, client);
  }
}

function Snippet({ text }: { text: string }) {
  const { t } = useI18n();
  const toast = useToast();
  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      toast(t("agent.setup.copied"), { tone: "success" });
    } catch {
      // The text is selectable: the user can copy it by hand.
    }
  }
  return (
    <div className="agent-snippet__row">
      <pre className="agent-snippet">{text}</pre>
      <div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            void copy();
          }}
        >
          {t("agent.setup.copy")}
        </Button>
      </div>
    </div>
  );
}

export function AgentSetup({ cliPath }: { cliPath: string | null }) {
  const { t } = useI18n();
  const [client, setClient] = useState<AgentClient>("claudeCode");
  return (
    <div className="setting-row setting-row--stacked">
      <h3 className="setting-row__label">{t("agent.setup.title")}</h3>
      <p className="field__hint">{t("agent.setup.body")}</p>
      {cliPath === null && <Callout tone="info">{t("agent.setup.cliMissing")}</Callout>}
      <Select
        label={t("agent.setup.pickClient")}
        value={client}
        options={AGENT_CLIENTS.map((each) => ({ value: each, label: t(`agent.setup.client.${each}`) }))}
        onChange={(event) => {
          setClient(event.target.value as AgentClient);
        }}
      />
      <p className="caption">{t("agent.setup.pairStep")}</p>
      <Snippet text={pairCommand(cliPath, client)} />
      <p className="caption">{t("agent.setup.configStep")}</p>
      <Snippet text={configFor(cliPath, client)} />
      <Callout tone="warning">{t("agent.setup.advice")}</Callout>
    </div>
  );
}
