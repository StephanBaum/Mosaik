/**
 * The SSH approval prompt (D18). Deny has the initial focus and Esc means Deny; the allow
 * buttons stay disabled for 1 s after the dialog opens and after the window regains focus,
 * so a keystroke meant for another window cannot approve a signature.
 */
import { useEffect, useRef, useState } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { useWindowFocused } from "../hooks/useWindowFocused.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { IpcError, SshApprovalRequest, SshDecision } from "../ipc/contract/index.ts";

export const ALLOW_GUARD_MS = 1000;
const TICK_MS = 250;
/** The countdown is announced to screen readers once, when this many seconds remain. */
const ANNOUNCE_AT_SECONDS = 10;

export function SshApprovalDialog() {
  const { ssh } = useEvents();
  const first = ssh[0];
  if (first === undefined) {
    return null;
  }
  // A new request restarts the dialog (and its guard).
  return <Prompt key={first.requestId} request={first} waiting={ssh.length - 1} />;
}

/** The clock and the guard: allow stays disabled for 1 s after opening and after a refocus. */
function useAllowGuard(): { now: number; locked: boolean } {
  const focused = useWindowFocused();
  const [now, setNow] = useState(() => Date.now());
  const [guardUntil, setGuardUntil] = useState(() => Date.now() + ALLOW_GUARD_MS);
  const wasFocused = useRef(focused);

  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, TICK_MS);
    return () => {
      clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    if (focused && !wasFocused.current) {
      setGuardUntil(Date.now() + ALLOW_GUARD_MS);
    }
    wasFocused.current = focused;
  }, [focused]);

  return { now, locked: !focused || now < guardUntil };
}

/** The countdown text for a live region: empty until `ANNOUNCE_AT_SECONDS`, then fixed. */
function useCountdownAnnouncement(seconds: number): string {
  const { t } = useI18n();
  const announced = useRef(false);
  const [announcement, setAnnouncement] = useState("");
  useEffect(() => {
    if (seconds <= ANNOUNCE_AT_SECONDS && !announced.current) {
      announced.current = true;
      setAnnouncement(t("ssh.expires", { seconds }));
    }
  }, [seconds, t]);
  return announcement;
}

function Prompt({ request, waiting }: { request: SshApprovalRequest; waiting: number }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { removeSsh } = useEvents();
  const { now, locked } = useAllowGuard();
  const [error, setError] = useState<IpcError | null>(null);

  async function answer(decision: SshDecision) {
    const result = await ipc.call("ssh_respond", { requestId: request.requestId, decision });
    if (result.ok || result.error.code === "sshRequestExpired") {
      removeSsh(request.requestId);
    } else {
      setError(result.error);
    }
  }

  const seconds = Math.max(0, Math.ceil((request.expiresAt - now) / 1000));
  const announcement = useCountdownAnnouncement(seconds);
  const client = request.client.processName ?? t("ssh.client.unknown");
  return (
    <Dialog
      title={t("ssh.title")}
      onClose={() => {
        void answer("deny");
      }}
    >
      <dl className="fields">
        <div className="fields__row">
          <dt>{t("ssh.key")}</dt>
          <dd>{request.keyTitle ?? t("vault.untitled")}</dd>
        </div>
        <div className="fields__row">
          <dt>{t("ssh.fingerprint")}</dt>
          <dd className="mono">{request.fingerprint}</dd>
        </div>
        <div className="fields__row">
          <dt>{t("ssh.client")}</dt>
          <dd>{request.client.pid === null ? client : `${client} (${String(request.client.pid)})`}</dd>
        </div>
      </dl>
      <p className="status">{t("ssh.expires", { seconds })}</p>
      <p role="status" className="visually-hidden">
        {announcement}
      </p>
      {waiting > 0 && <p className="status">{t("ssh.queue", { count: waiting })}</p>}
      <ErrorAlert error={error} />
      <div className="actions">
        <Button
          autoFocus
          onClick={() => {
            void answer("deny");
          }}
        >
          {t("ssh.deny")}
        </Button>
        <Button
          disabled={locked}
          onClick={() => {
            void answer("allowOnce");
          }}
        >
          {t("ssh.allowOnce")}
        </Button>
        {request.canRemember && (
          <Button
            disabled={locked}
            onClick={() => {
              void answer("allowFor15Minutes");
            }}
          >
            {t("ssh.allowFor15Minutes")}
          </Button>
        )}
      </div>
    </Dialog>
  );
}
