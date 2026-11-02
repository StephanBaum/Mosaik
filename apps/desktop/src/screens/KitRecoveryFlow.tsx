/**
 * Shown when an unlocked account has not confirmed its Emergency Kit (the app was closed
 * or locked during onboarding): save the kit again with the master password, then
 * confirm the last group of the Secret Key.
 */
import { useState } from "react";
import { AuthShell } from "../components/AuthShell.tsx";
import { Button } from "../components/Button.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { Icon } from "../components/icons.tsx";
import { ReauthDialog } from "../components/ReauthDialog.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { IpcError, SessionState } from "../ipc/contract/index.ts";
import { kitLabels } from "./onboarding/kit.ts";
import { VerifyStep } from "./onboarding/VerifyStep.tsx";

export function KitRecoveryFlow({ onDone }: { onDone: (session: SessionState) => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [asking, setAsking] = useState(false);
  const [saved, setSaved] = useState(false);

  async function save(password: string): Promise<IpcError | null> {
    const result = await ipc.call("save_emergency_kit", { labels: kitLabels(t), password });
    if (!result.ok) {
      return result.error;
    }
    setSaved(result.value.kind === "saved");
    return null;
  }

  if (saved) {
    return (
      <VerifyStep
        step={3}
        onVerified={() => {
          void ipc.call("session_state", null).then((result) => {
            if (result.ok) {
              onDone(result.value);
            }
          });
        }}
      />
    );
  }
  return (
    <AuthShell>
      <div className="auth__main">
        <div className="auth__intro">
          <h1 className="display">{t("kit.recover.title")}</h1>
          <p className="lede">{t("kit.recover.body")}</p>
        </div>
        <div className="auth__actions">
          <HoloButton
            size="lg"
            block
            onClick={() => {
              setAsking(true);
            }}
          >
            <Icon name="download" />
            {t("kit.save")}
          </HoloButton>
          <Button
            variant="ghost"
            size="lg"
            block
            className="button--muted"
            onClick={() => {
              setSaved(true);
            }}
          >
            {t("kit.recover.alreadySaved")}
          </Button>
        </div>
      </div>
      {asking && (
        <ReauthDialog
          kit
          onSubmit={save}
          onClose={() => {
            setAsking(false);
          }}
        />
      )}
    </AuthShell>
  );
}
