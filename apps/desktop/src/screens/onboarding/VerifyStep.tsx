/** Step 3 (D04): type the last group of the Secret Key to confirm the kit was saved. */
import { useState, type SyntheticEvent } from "react";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { IpcError } from "../../ipc/contract/index.ts";
import { MASKED_KEY_PREFIX } from "./kit.ts";
import { OnboardingShell } from "./OnboardingShell.tsx";

export function VerifyStep({
  step = 3,
  onVerified,
}: {
  step?: number;
  onVerified: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [lastGroup, setLastGroup] = useState("");
  const [mistyped, setMistyped] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setError(null);
    const result = await ipc.call("verify_emergency_kit", { lastGroup });
    setLastGroup("");
    if (!result.ok) {
      setError(result.error);
    } else if (result.value.matches) {
      onVerified();
    } else {
      setMistyped(true);
    }
  }

  return (
    <OnboardingShell step={step}>
      <div className="auth__main">
        <div className="auth__intro">
          <h1 className="display">{t("onboarding.secretKey.title")}</h1>
          <p className="lede">{t("onboarding.secretKey.body")}</p>
        </div>
        <div className="auth__intro">
          <span className="row__label">{t("kit.secretKey")}</span>
          <p className="masked-key" aria-hidden="true">
            <span className="muted">{MASKED_KEY_PREFIX}</span>
            <span className="masked-key__slot">____</span>
          </p>
        </div>
        <form className="form-stack" onSubmit={(event) => void submit(event)} noValidate>
          <TextField
            size="lg"
            label={t("onboarding.kit.lastGroup")}
            className="field__input--code"
            mono
            maxLength={4}
            autoCapitalize="characters"
            autoComplete="off"
            spellCheck={false}
            error={mistyped ? t("onboarding.secretKey.mistyped") : undefined}
            value={lastGroup}
            onChange={(event) => {
              setLastGroup(event.target.value);
              setMistyped(false);
            }}
          />
          <Callout tone="info">{t("onboarding.secretKey.verify")}</Callout>
          <ErrorAlert error={error} />
          <HoloButton type="submit" size="lg" block>
            {t("onboarding.kit.confirm")}
          </HoloButton>
        </form>
      </div>
    </OnboardingShell>
  );
}
