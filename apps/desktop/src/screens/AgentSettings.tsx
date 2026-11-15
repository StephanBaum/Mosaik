/**
 * The "AI agents" group of the settings (ADR-0159): the switch, the paired agents, the open
 * permissions, "Stop all agent access now", the setup help and the activity. The status is read
 * again whenever Rust reports a change (`agentStateChanged`). Nothing here shows or sends a
 * value; agents get a key only through the dialogs and the sites listed in the item editor.
 */
import { useEffect, useState } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { Switch } from "../components/Switch.tsx";
import { UserText } from "../components/UserText.tsx";
import { formatRelative } from "../i18n/format.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type {
  AgentAccessStatus,
  AgentGrantView,
  AgentPairingView,
  IpcError,
} from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { AgentActivity } from "./AgentActivity.tsx";
import { AgentSetup } from "./AgentSetup.tsx";
import { SettingsSection } from "./SettingsSection.tsx";

function PairingRow({
  pairing,
  now,
  confirming,
  onAsk,
  onCancel,
  onConfirm,
}: {
  pairing: AgentPairingView;
  now: number;
  confirming: boolean;
  onAsk: () => void;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t, locale } = useI18n();
  if (confirming) {
    return (
      <li className="row">
        <div className="row__main" role="alert">
          <p>{t("agent.settings.removeTitle", { name: pairing.name })}</p>
          <p className="caption">{t("agent.settings.removeBody")}</p>
        </div>
        <Button variant="danger" size="sm" autoFocus onClick={onConfirm}>
          {t("agent.settings.remove")}
        </Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>
          {t("action.cancel")}
        </Button>
      </li>
    );
  }
  const used =
    pairing.lastUsedAt === null
      ? t("agent.settings.neverUsed")
      : t("agent.settings.lastUsed", { when: formatRelative(locale, pairing.lastUsedAt, now) });
  return (
    <li className="row">
      <div className="row__main">
        <p>
          <UserText>{pairing.name}</UserText>
        </p>
        <p className="caption">
          {t("agent.settings.pairedAt", { when: formatRelative(locale, pairing.pairedAt, now) })}
          {" · "}
          {used}
        </p>
      </div>
      <Button
        variant="ghost"
        size="sm"
        aria-label={t("agent.settings.removeNamed", { name: pairing.name })}
        onClick={onAsk}
      >
        {t("agent.settings.remove")}
      </Button>
    </li>
  );
}

function GrantRow({
  grant,
  now,
  onRevoke,
}: {
  grant: AgentGrantView;
  now: number;
  onRevoke: () => void;
}) {
  const { t, locale } = useI18n();
  const until = {
    once: t("agent.settings.grantOnce"),
    until: t("agent.settings.grantUntil", {
      when: formatRelative(locale, grant.expiresAt ?? now, now),
    }),
    untilLock: t("agent.settings.grantUntilLock"),
  }[grant.state];
  return (
    <li className="row">
      <div className="row__main">
        <p>
          <UserText>
            {t("agent.settings.grantFor", { client: grant.pairingName, item: grant.itemTitle })}
          </UserText>
        </p>
        <p className="caption">
          <UserText>{grant.fieldLabel}</UserText>
          {" · "}
          {until}
        </p>
      </div>
      <Button
        variant="ghost"
        size="sm"
        aria-label={t("agent.settings.revokeNamed", { client: grant.pairingName, item: grant.itemTitle })}
        onClick={onRevoke}
      >
        {t("agent.settings.revoke")}
      </Button>
    </li>
  );
}

/** "Stop all agent access now": one question, then it is done. */
function StopAll({ onStop }: { onStop: () => void }) {
  const { t } = useI18n();
  const [asking, setAsking] = useState(false);
  if (!asking) {
    return (
      <div className="setting-row">
        <Button
          variant="danger"
          size="sm"
          onClick={() => {
            setAsking(true);
          }}
        >
          {t("agent.settings.stopAll")}
        </Button>
      </div>
    );
  }
  return (
    <div className="setting-row">
      <div role="alert">
        <p>{t("agent.settings.stopAllTitle")}</p>
        <p className="caption">{t("agent.settings.stopAllBody")}</p>
      </div>
      <Button
        variant="danger"
        size="sm"
        autoFocus
        onClick={() => {
          setAsking(false);
          onStop();
        }}
      >
        {t("agent.settings.stopAllConfirm")}
      </Button>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => {
          setAsking(false);
        }}
      >
        {t("action.cancel")}
      </Button>
    </div>
  );
}

type StatusCall = Promise<{ ok: true; value: AgentAccessStatus } | { ok: false; error: IpcError }>;

function Lists({ status, apply }: { status: AgentAccessStatus; apply: (call: StatusCall) => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [confirming, setConfirming] = useState<string | null>(null);
  const [now] = useState(() => Date.now());
  return (
    <>
      {status.pairings.length === 0 ? (
        <div className="setting-row">
          <p className="field__hint">{t("agent.settings.noClients")}</p>
        </div>
      ) : (
        <ul className="rows" aria-label={t("agent.settings.clients")}>
          {status.pairings.map((pairing) => (
            <PairingRow
              key={pairing.pairingId}
              pairing={pairing}
              now={now}
              confirming={confirming === pairing.pairingId}
              onAsk={() => {
                setConfirming(pairing.pairingId);
              }}
              onCancel={() => {
                setConfirming(null);
              }}
              onConfirm={() => {
                setConfirming(null);
                apply(ipc.call("agent_pairing_remove", { pairingId: pairing.pairingId }));
              }}
            />
          ))}
        </ul>
      )}
      <h3 className="setting-row__label">{t("agent.settings.grants")}</h3>
      {status.grants.length === 0 ? (
        <div className="setting-row">
          <p className="field__hint">{t("agent.settings.noGrants")}</p>
        </div>
      ) : (
        <ul className="rows" aria-label={t("agent.settings.grants")}>
          {status.grants.map((grant) => (
            <GrantRow
              key={grant.grantId}
              grant={grant}
              now={now}
              onRevoke={() => {
                apply(ipc.call("agent_grant_revoke", { grantId: grant.grantId }));
              }}
            />
          ))}
        </ul>
      )}
      <StopAll
        onStop={() => {
          apply(ipc.call("agent_stop_all", null));
        }}
      />
    </>
  );
}

export function AgentSettings({ onError }: { onError: (error: IpcError | null) => void }) {
  const { t, errorMessage } = useI18n();
  const ipc = useIpc();
  const { agent } = useEvents();
  const { version } = agent;
  const [status, setStatus] = useState<AgentAccessStatus | null>(null);

  useEffect(() => {
    let current = true;
    void ipc.call("agent_access_status", null).then((result) => {
      if (current && result.ok) {
        setStatus(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, version]);

  /** Shows the status a command returned, or the error and the status as it is now. */
  function apply(call: StatusCall) {
    void call.then(async (result) => {
      onError(result.ok ? null : result.error);
      const fresh = result.ok ? result : await ipc.call("agent_access_status", null);
      if (fresh.ok) {
        setStatus(fresh.value);
      }
    });
  }

  return (
    <SettingsSection title={t("agent.settings.title")} body={t("agent.settings.body")}>
      <div className="setting-row">
        <Switch
          label={t("agent.settings.enable")}
          hint={t("agent.settings.hint")}
          checked={status?.enabled ?? false}
          onChange={(event) => {
            apply(ipc.call("agent_access_set_enabled", { enabled: event.target.checked }));
          }}
        />
      </div>
      {status?.enabled === true && status.endpoint === "unavailable" && (
        <div className="setting-row">
          <p className="alert" role="alert">
            {errorMessage({ code: "agentEndpointUnavailable" })}
          </p>
        </div>
      )}
      {status?.enabled === true && (
        <>
          <Lists status={status} apply={apply} />
          <AgentSetup cliPath={status.cliPath} />
          <AgentActivity status={status} />
        </>
      )}
    </SettingsSection>
  );
}
