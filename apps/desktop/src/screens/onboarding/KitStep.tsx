/**
 * Step 2 (D03): the Secret Key and the Emergency Kit PDF. Rust shows the save dialog and
 * writes the file; there is no print button (the webview has no print permission).
 */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { LogoMark } from "../../components/LogoMark.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { EmergencyKit, IpcError } from "../../ipc/contract/index.ts";
import { kitLabels } from "./kit.ts";
import { OnboardingShell } from "./OnboardingShell.tsx";

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

  const isSaved = saved === "saved";
  const saveButton = isSaved ? (
    <Button size="lg" onClick={() => void save()}>
      <Icon name="download" />
      {t("kit.save")}
    </Button>
  ) : (
    <HoloButton size="lg" onClick={() => void save()}>
      <Icon name="download" />
      {t("kit.save")}
    </HoloButton>
  );
  return (
    <OnboardingShell step={2}>
      <div className="auth__main">
        <div className="auth__intro">
          <h1 className="display">{t("kit.title")}</h1>
          <p className="lede">{t("kit.body")}</p>
        </div>
        <KitCard kit={kit} />
        <div className="auth__actions">
          {saveButton}
          <p className="caption" role="status">
            {saved === "saved" ? t("kit.saved") : saved === "cancelled" ? t("kit.cancelled") : ""}
          </p>
          <ErrorAlert error={error} />
        </div>
        <Callout tone="warning">{t("kit.warning")}</Callout>
        <div className="auth__actions">
          {isSaved ? (
            <HoloButton size="lg" block onClick={onContinue}>
              {t("action.continue")}
            </HoloButton>
          ) : (
            <>
              <Button size="lg" block disabled>
                {t("action.continue")}
              </Button>
              <p className="caption auth__note">{t("kit.continueHint")}</p>
            </>
          )}
        </div>
      </div>
    </OnboardingShell>
  );
}

function KitCard({ kit }: { kit: EmergencyKit }) {
  const { t } = useI18n();
  const labels = kitLabels(t);
  return (
    <figure className="kit-card" aria-label={t("kit.preview")}>
      <div className="kit-card__header">
        <LogoMark size="sm" />
        <span className="kit-card__brand">{t("app.name")}</span>
        <span className="kit-card__title">{labels.title}</span>
      </div>
      <dl className="kit">
        <dt>{labels.signInUrl}</dt>
        <dd>{labels.signInUrlValue}</dd>
        <dt>{t("onboarding.create.email")}</dt>
        <dd>{kit.email}</dd>
        <dt>{t("onboarding.kit.accountId")}</dt>
        <dd className="mono">{kit.accountId}</dd>
        <dt className="kit__secret-label">{t("kit.secretKey")}</dt>
        <dd className="secret-key secret-key-box">{kit.secretKey}</dd>
        <dt>{labels.masterPassword}</dt>
        <dd>
          <span className="kit__blank" role="img" aria-label={labels.masterPasswordHint} />
        </dd>
      </dl>
      <p className="caption kit-card__footer">{labels.footer}</p>
    </figure>
  );
}
