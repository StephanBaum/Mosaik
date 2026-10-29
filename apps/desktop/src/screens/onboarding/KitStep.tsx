/**
 * Step 2 (D03): the Secret Key and the Emergency Kit PDF. Rust shows the save dialog and
 * writes the file; there is no print button (the webview has no print permission).
 */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { EmergencyKit, IpcError } from "../../ipc/contract/index.ts";
import { kitLabels } from "./kit.ts";
import { TOTAL_STEPS } from "./WelcomeStep.tsx";

export function KitStep({ kit, onContinue }: { kit: EmergencyKit; onContinue: () => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [saved, setSaved] = useState<"unknown" | "saved" | "cancelled">("unknown");
  const [error, setError] = useState<IpcError | null>(null);

  async function save() {
    setError(null);
    const result = await ipc.call("save_emergency_kit", { labels: kitLabels(t), password: null });
    if (result.ok) {
      setSaved(result.value.kind);
    } else {
      setError(result.error);
    }
  }

  return (
    <main id="main" className="screen screen--centered">
      <div className="panel">
        <p className="eyebrow">{t("onboarding.step", { current: 2, total: TOTAL_STEPS })}</p>
        <h1 className="title">{t("kit.title")}</h1>
        <p className="body muted">{t("kit.body")}</p>
        <dl className="kit">
          <dt>{t("onboarding.create.email")}</dt>
          <dd>{kit.email}</dd>
          <dt>{t("onboarding.kit.accountId")}</dt>
          <dd className="mono">{kit.accountId}</dd>
          <dt>{t("kit.secretKey")}</dt>
          <dd className="mono secret-key">{kit.secretKey}</dd>
        </dl>
        <Button
          onClick={() => {
            void save();
          }}
        >
          {t("kit.save")}
        </Button>
        <p className="status" role="status">
          {saved === "saved" ? t("kit.saved") : saved === "cancelled" ? t("kit.cancelled") : ""}
        </p>
        <ErrorAlert error={error} />
        <HoloButton disabled={saved !== "saved"} onClick={onContinue}>
          {t("action.continue")}
        </HoloButton>
      </div>
    </main>
  );
}
