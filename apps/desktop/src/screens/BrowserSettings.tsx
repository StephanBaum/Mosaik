/**
 * The "Browsers" group of the settings (M4): the paired browser extensions, whether each is
 * connected or when it was last used, and Disconnect with a confirmation. The list is read
 * again whenever Rust reports a change (`browserPairingsChanged`).
 */
import { useState } from "react";
import { Button } from "../components/Button.tsx";
import { UserText } from "../components/UserText.tsx";
import { formatRelative } from "../i18n/format.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { BrowserPairing, IpcError } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { SettingsSection } from "./SettingsSection.tsx";
import { useBrowserStatus } from "./useBrowserStatus.ts";

function PairingRow({
  pairing,
  now,
  confirming,
  onAsk,
  onCancel,
  onConfirm,
}: {
  pairing: BrowserPairing;
  /** The time the list is relative to. */
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
          <p>
            {t("browser.settings.disconnectTitle", { name: pairing.label })}
          </p>
          <p className="caption">{t("browser.settings.disconnectBody")}</p>
        </div>
        <Button variant="danger" size="sm" autoFocus onClick={onConfirm}>
          {t("browser.settings.disconnect")}
        </Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>
          {t("action.cancel")}
        </Button>
      </li>
    );
  }
  const when =
    pairing.lastUsedAt === null
      ? t("browser.settings.neverUsed")
      : t("browser.settings.lastUsed", {
          when: formatRelative(locale, pairing.lastUsedAt, now),
        });
  return (
    <li className="row">
      <div className="row__main">
        <p>
          <UserText>{pairing.label}</UserText>
        </p>
        <p className="caption">
          {t(`browser.family.${pairing.browser}`)}
          {" · "}
          {pairing.connected ? t("browser.settings.connected") : when}
        </p>
      </div>
      <Button
        variant="ghost"
        size="sm"
        aria-label={t("browser.settings.disconnectNamed", { name: pairing.label })}
        onClick={onAsk}
      >
        {t("browser.settings.disconnect")}
      </Button>
    </li>
  );
}

export function BrowserSettings({ onError }: { onError: (error: IpcError | null) => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [status, setStatus] = useBrowserStatus();
  const [confirming, setConfirming] = useState<string | null>(null);
  const [now] = useState(() => Date.now());

  async function remove(pairingId: string) {
    const result = await ipc.call("browser_remove_pairing", { pairingId });
    onError(result.ok ? null : result.error);
    setConfirming(null);
    if (result.ok) {
      setStatus(result.value);
    } else {
      // Gone already: show the list as it is now.
      const fresh = await ipc.call("browser_status", null);
      if (fresh.ok) {
        setStatus(fresh.value);
      }
    }
  }

  return (
    <SettingsSection title={t("browser.settings.title")} body={t("browser.settings.body")}>
      {status !== null && status.endpoint !== "running" && (
        <div className="setting-row">
          <p className="alert" role="alert">
            {t(
              status.endpoint === "inUse"
                ? "browser.settings.endpointInUse"
                : "browser.settings.endpointUnavailable",
            )}
          </p>
        </div>
      )}
      {status !== null && status.registry === "unreadable" && (
        <div className="setting-row">
          <p className="alert" role="alert">
            {t("browser.settings.registryUnreadable")}
          </p>
        </div>
      )}
      {status !== null && status.pairings.length === 0 && (
        <div className="setting-row">
          <p className="field__hint">{t("browser.settings.empty")}</p>
        </div>
      )}
      {status !== null && status.pairings.length > 0 && (
        <ul className="rows">
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
                void remove(pairing.pairingId);
              }}
            />
          ))}
        </ul>
      )}
    </SettingsSection>
  );
}
