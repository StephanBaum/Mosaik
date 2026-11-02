/** The SSH agent group of the settings: switch, endpoint, status and the keys not served. */
import { useEffect, useState } from "react";
import { Button } from "../components/Button.tsx";
import { Switch } from "../components/Switch.tsx";
import { useToast } from "../components/ToastProvider.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { IpcError, SshAgentStatus } from "../ipc/contract/index.ts";
import { SettingsSection } from "./SettingsSection.tsx";

export function SshSettings({ onError }: { onError: (error: IpcError | null) => void }) {
  const { t, errorMessage } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [ssh, setSsh] = useState<SshAgentStatus | null>(null);

  useEffect(() => {
    void ipc.call("ssh_agent_status", null).then((result) => {
      if (result.ok) {
        setSsh(result.value);
      }
    });
  }, [ipc]);

  async function setAgent(enabled: boolean) {
    const result = await ipc.call("ssh_agent_set_enabled", { enabled });
    onError(result.ok ? null : result.error);
    const status = result.ok ? result : await ipc.call("ssh_agent_status", null);
    if (status.ok) {
      setSsh(status.value);
    }
  }

  async function copyEndpoint(endpoint: string) {
    try {
      await navigator.clipboard.writeText(endpoint);
    } catch {
      toast(errorMessage({ code: "clipboardUnavailable" }));
    }
  }

  const endpoint = ssh?.endpoint ?? null;
  return (
    <SettingsSection title={t("settings.ssh")} body={t("settings.ssh.body")}>
      <div className="setting-row">
        <Switch
          label={t("settings.ssh.enable")}
          hint={t("settings.ssh.hint")}
          checked={ssh?.enabled ?? false}
          onChange={(event) => {
            void setAgent(event.target.checked);
          }}
        />
      </div>
      {ssh?.running === true && (
        <div className="setting-row">
          <p className="setting-row__label">{t("settings.ssh.status", { count: ssh.keyCount })}</p>
        </div>
      )}
      {endpoint !== null && (
        <div className="setting-row">
          <span className="setting-row__label">{t("settings.ssh.endpoint")}</span>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              void copyEndpoint(endpoint);
            }}
          >
            {t("action.copy")}
          </Button>
          <code className="mono setting-row__code">{endpoint}</code>
        </div>
      )}
      {ssh?.error != null && (
        <div className="setting-row">
          <p className="alert" role="alert">
            {errorMessage({ code: ssh.error })}
          </p>
        </div>
      )}
      {ssh !== null && ssh.skipped.length > 0 && (
        <div className="setting-row">
          <h3 className="row__label">{t("settings.ssh.skipped")}</h3>
          <ul className="setting-row__code">
            {ssh.skipped.map((key) => (
              <li key={key.itemId}>{t(`ssh.skip.${key.reason}`)}</li>
            ))}
          </ul>
        </div>
      )}
    </SettingsSection>
  );
}
