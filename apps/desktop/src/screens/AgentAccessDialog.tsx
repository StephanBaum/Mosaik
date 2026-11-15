/**
 * The agent access prompt (ADR-0159, spec section 9.2, builder check B3): an agent asks to use a
 * key. Facts first (who, which item and key, where the value may go, how it is sent), then the
 * agent's own words in a labelled quote block that never looks like Mosaic's text. Deny has the
 * initial focus and Esc means Deny; the allow buttons stay disabled for 1 s after the dialog is
 * shown, after the window regains focus and after the dialog shows another request (a new
 * request ID is a new dialog with its own second). There is no control that edits a policy or
 * adds a destination: that is the item editor's job (D8).
 */
import { useState, type ReactNode } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { Icon } from "../components/icons.tsx";
import { UserText } from "../components/UserText.tsx";
import { useAllowGuard } from "../hooks/useAllowGuard.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { AgentAccessDecision, AgentAccessRequest, IpcError } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { AgentExpiry } from "./AgentPairingDialog.tsx";
import { isolate } from "./sync/hooks.ts";

export function AgentAccessDialog() {
  const { agent } = useEvents();
  if (agent.access === null) {
    return null;
  }
  // A new request restarts the dialog (and its guard); the same request with a new queue
  // length only updates it.
  return <Prompt key={agent.access.requestId} request={agent.access} />;
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="row">
      <dt className="row__label">{label}</dt>
      <dd className="row__value">{children}</dd>
    </div>
  );
}

function Facts({ request }: { request: AgentAccessRequest }) {
  const { t } = useI18n();
  return (
    <dl className="rows rows--closed">
      <Row label={t("agent.access.client")}>
        <UserText>{request.pairingName}</UserText>
        <p className="caption">{t("agent.access.clientNote", { name: isolate(request.pairingName) })}</p>
      </Row>
      <Row label={t("agent.access.item")}>
        <UserText>{request.itemTitle}</UserText>
      </Row>
      <Row label={t("agent.access.vault")}>
        <UserText>{request.vaultName}</UserText>
      </Row>
      <Row label={t("agent.access.field")}>
        <UserText>{request.fieldLabel}</UserText>
      </Row>
      <Row label={t("agent.access.destinations")}>
        <ul className="agent-sites">
          {request.origins.map((origin) => (
            <li key={origin} className="mono" translate="no">
              {origin}
            </li>
          ))}
        </ul>
      </Row>
      <Row label={t("agent.access.how")}>{t(`agent.access.inject.${request.inject}`)}</Row>
    </dl>
  );
}

/** What the agent wrote, as plain text in its own labelled block (never as markup). */
function Reason({ reason }: { reason: string }) {
  const { t } = useI18n();
  return (
    <figure className="agent-reason">
      <figcaption className="caption">{t("agent.access.reasonLabel")}</figcaption>
      <blockquote className="agent-reason__text">
        <UserText>{reason}</UserText>
      </blockquote>
      <p className="caption">{t("agent.access.reasonNote")}</p>
    </figure>
  );
}

const ALLOW_ORDER: readonly Exclude<AgentAccessDecision, "deny">[] = ["allowOnce", "allow15m", "allow1h", "allowUntilLock"];
const ALLOW_LABEL = {
  allowOnce: "agent.access.allowOnce",
  allow15m: "agent.access.allow15m",
  allow1h: "agent.access.allow1h",
  allowUntilLock: "agent.access.allowUntilLock",
} as const;

function Prompt({ request }: { request: AgentAccessRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { agent } = useEvents();
  const { now, locked } = useAllowGuard();
  const [error, setError] = useState<IpcError | null>(null);

  async function answer(decision: AgentAccessDecision) {
    const result = await ipc.call("agent_access_respond", { requestId: request.requestId, decision });
    if (result.ok || result.error.code === "agentRequestExpired") {
      agent.removeAccess(request.requestId);
    } else {
      // `agentAllowTooSoon` keeps the dialog open.
      setError(result.error);
    }
  }

  return (
    <Dialog
      title={t("agent.access.title")}
      icon={
        <span className="key-tile">
          <Icon name="key" size="lg" />
        </span>
      }
      onClose={() => {
        void answer("deny");
      }}
    >
      <div className="browser-prompt agent-prompt">
        <p className="lede">{t("agent.access.description")}</p>
        <Facts request={request} />
        {request.reason !== null && request.reason !== "" && <Reason reason={request.reason} />}
        <AgentExpiry expiresAt={request.expiresAt} now={now} />
        {request.waiting > 0 && <p className="status">{t("agent.access.queue", { count: request.waiting })}</p>}
        <ErrorAlert error={error} />
        <div className="agent-prompt__actions">
          <Button
            data-autofocus=""
            onClick={() => {
              void answer("deny");
            }}
          >
            {t("agent.access.deny")}
          </Button>
          {ALLOW_ORDER.map((decision, index) => {
            const Control = index === 0 ? HoloButton : Button;
            return (
              <Control
                key={decision}
                disabled={locked}
                onClick={() => {
                  void answer(decision);
                }}
              >
                {t(ALLOW_LABEL[decision])}
              </Control>
            );
          })}
        </div>
      </div>
    </Dialog>
  );
}
