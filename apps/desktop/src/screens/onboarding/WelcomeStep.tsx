/** Step 1 (D02): create the account, or join an existing one ("I already use Mosaic"). */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { CreatedAccount, IpcError } from "../../ipc/contract/index.ts";
import { OnboardingShell } from "./OnboardingShell.tsx";

function useCreateAccount(onCreated: (created: CreatedAccount) => void) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [fields, setFields] = useState({ email: "", password: "", confirm: "" });
  const [busy, setBusy] = useState(false);
  const [mismatch, setMismatch] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setError(null);
    if (fields.password !== fields.confirm) {
      setMismatch(true);
      return;
    }
    setMismatch(false);
    setBusy(true);
    const result = await ipc.call("create_account", {
      email: fields.email,
      password: fields.password,
      defaultVaultName: t("vault.defaultName"),
    });
    setBusy(false);
    if (result.ok) {
      setFields({ ...fields, password: "", confirm: "" });
      onCreated(result.value);
    } else {
      setError(result.error);
    }
  }

  return { fields, setFields, busy, mismatch, error, submit };
}

export function WelcomeStep({
  onCreated,
  onJoin,
}: {
  onCreated: (created: CreatedAccount) => void;
  /** Opens "I already use Mosaic": add this device to an existing account. */
  onJoin: () => void;
}) {
  const { t } = useI18n();
  const { fields, setFields, busy, mismatch, error, submit } = useCreateAccount(onCreated);

  return (
    <OnboardingShell step={1}>
      <div className="auth__main">
        <div className="auth__intro">
          <h1 className="display">{t("onboarding.create.title")}</h1>
          <p className="lede">{t("onboarding.create.body")}</p>
        </div>
        <form className="form-stack" onSubmit={(event) => void submit(event)} noValidate>
          <TextField
            size="lg"
            label={t("onboarding.create.email")}
            type="email"
            autoComplete="email"
            required
            value={fields.email}
            onChange={(event) => {
              setFields({ ...fields, email: event.target.value });
            }}
          />
          <PasswordField
            size="lg"
            toggleLabel="password"
            label={t("onboarding.create.password")}
            hint={t("onboarding.create.passwordHint")}
            autoComplete="new-password"
            required
            value={fields.password}
            onChange={(event) => {
              setFields({ ...fields, password: event.target.value });
            }}
          />
          <PasswordField
            size="lg"
            toggleLabel="password"
            label={t("onboarding.create.confirm")}
            autoComplete="new-password"
            required
            error={mismatch ? t("onboarding.create.mismatch") : undefined}
            value={fields.confirm}
            onChange={(event) => {
              setFields({ ...fields, confirm: event.target.value });
            }}
          />
          <ErrorAlert error={error} />
          <p className="status" aria-live="polite">
            {busy ? t("onboarding.create.working") : ""}
          </p>
          <HoloButton type="submit" size="lg" block disabled={busy}>
            {t("onboarding.create.submit")}
          </HoloButton>
        </form>
        <Button variant="ghost" size="lg" block className="button--muted" onClick={onJoin}>
          {t("onboarding.join.link")}
        </Button>
        <p className="caption auth__note">{t("app.preAudit")}</p>
      </div>
    </OnboardingShell>
  );
}
