/**
 * The agent pairing prompt (ADR-0159): a program on this computer asked to connect (`mosaic agent
 * pair`). The terminal shows the same code; the user checks that they match and that they just
 * ran the command. Deny has the initial focus and Esc means Deny; Allow stays disabled for 1 s
 * after the dialog opens and after the window regains focus. The name is what the program typed:
 * it is shown as plain text and the dialog says Mosaic cannot check it.
 */
import { useEffect, useRef, useState } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { Callout } from "../components/Callout.tsx";
import { CountdownRing } from "../components/CountdownRing.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { Icon } from "../components/icons.tsx";
import { Kbd } from "../components/Kbd.tsx";
import { UserText } from "../components/UserText.tsx";
import { useAllowGuard } from "../hooks/useAllowGuard.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { AgentPairingRequest, IpcError, PairingDecision } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { formatCode } from "./BrowserPairingDialog.tsx";

/** The countdown is announced to screen readers once, when this many seconds remain. */
const ANNOUNCE_AT_SECONDS = 10;

/** The ring, the countdown text and the Esc hint, with the live region for screen readers. */
export function AgentExpiry({ expiresAt, now }: { expiresAt: number; now: number }) {
  const { t } = useI18n();
  const [total] = useState(() => Math.max(1, Math.ceil((expiresAt - Date.now()) / 1000)));
  const seconds = Math.max(0, Math.ceil((expiresAt - now) / 1000));
  const announced = useRef(false);
  const [announcement, setAnnouncement] = useState("");
  useEffect(() => {
    if (seconds <= ANNOUNCE_AT_SECONDS && !announced.current) {
      announced.current = true;
      setAnnouncement(t("agent.expires", { seconds }));
    }
  }, [seconds, t]);
  return (
    <>
      <div className="browser-prompt__expiry caption">
        <CountdownRing remaining={seconds} total={total} />
        <p className="browser-prompt__expiry-text">{t("agent.expires", { seconds })}</p>
        <span>
          <Kbd>{t("keys.esc")}</Kbd> {t("agent.escDenies")}
        </span>
      </div>
      <p role="status" className="visually-hidden">
        {announcement}
      </p>
    </>
  );
}

export function AgentPairingDialog() {
  const { agent } = useEvents();
  if (agent.pairing === null) {
    return null;
  }
  // A new request restarts the dialog (and its guard).
  return <Prompt key={agent.pairing.requestId} request={agent.pairing} />;
}

function Prompt({ request }: { request: AgentPairingRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { agent } = useEvents();
  const { now, locked } = useAllowGuard();
  const [error, setError] = useState<IpcError | null>(null);

  async function answer(decision: PairingDecision) {
    const result = await ipc.call("agent_pairing_respond", { requestId: request.requestId, decision });
    if (result.ok || result.error.code === "agentRequestExpired") {
      agent.removePairing(request.requestId);
    } else {
      // `agentAllowTooSoon` keeps the dialog open.
      setError(result.error);
    }
  }

  return (
    <Dialog
      title={t("agent.pair.title")}
      icon={
        <span className="key-tile">
          <Icon name="shield" size="lg" />
        </span>
      }
      onClose={() => {
        void answer("deny");
      }}
    >
      <div className="browser-prompt">
        <p className="lede">{t("agent.pair.description")}</p>
        <div className="browser-prompt__code-box">
          <span className="caption">{t("agent.pair.code")}</span>
          <p className="browser-prompt__code" translate="no">
            {formatCode(request.code)}
          </p>
          <span className="caption">
            {t("agent.pair.client")}: <UserText>{request.name}</UserText>
          </span>
        </div>
        <Callout tone="warning">{t("agent.pair.warning")}</Callout>
        <AgentExpiry expiresAt={request.expiresAt} now={now} />
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button
            data-autofocus=""
            onClick={() => {
              void answer("deny");
            }}
          >
            {t("agent.pair.deny")}
          </Button>
          <HoloButton
            disabled={locked}
            onClick={() => {
              void answer("allow");
            }}
          >
            {t("agent.pair.allow")}
          </HoloButton>
        </div>
      </div>
    </Dialog>
  );
}
