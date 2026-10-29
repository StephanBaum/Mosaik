/** Step 3 (D04): type the last group of the Secret Key to confirm the kit was saved. */
import { useState, type SyntheticEvent } from "react";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { IpcError } from "../../ipc/contract/index.ts";
import { TOTAL_STEPS } from "./WelcomeStep.tsx";

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
    <main id="main" className="screen screen--centered">
      <form className="panel" onSubmit={(event) => void submit(event)} noValidate>
        <p className="eyebrow">{t("onboarding.step", { current: step, total: TOTAL_STEPS })}</p>
        <h1 className="title">{t("onboarding.secretKey.title")}</h1>
        <p className="body muted">{t("onboarding.secretKey.body")}</p>
        <p className="body">{t("onboarding.secretKey.verify")}</p>
        <TextField
          label={t("onboarding.kit.lastGroup")}
          mono
          autoComplete="off"
          spellCheck={false}
          aria-invalid={mistyped}
          value={lastGroup}
          onChange={(event) => {
            setLastGroup(event.target.value);
            setMistyped(false);
          }}
        />
        {mistyped && (
          <p className="alert" role="alert">
            {t("onboarding.secretKey.mistyped")}
          </p>
        )}
        <ErrorAlert error={error} />
        <HoloButton type="submit">{t("onboarding.kit.confirm")}</HoloButton>
      </form>
    </main>
  );
}
