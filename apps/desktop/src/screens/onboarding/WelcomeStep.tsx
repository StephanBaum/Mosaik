/** Step 1 (D02): create the account. */
import { useState, type SyntheticEvent } from "react";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { CreatedAccount, IpcError } from "../../ipc/contract/index.ts";

export const TOTAL_STEPS = 5;

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

export function WelcomeStep({ onCreated }: { onCreated: (created: CreatedAccount) => void }) {
  const { t } = useI18n();
  const { fields, setFields, busy, mismatch, error, submit } = useCreateAccount(onCreated);

  return (
    <main id="main" className="screen screen--centered">
      <form className="panel" onSubmit={(event) => void submit(event)} noValidate>
        <p className="eyebrow">{t("onboarding.step", { current: 1, total: TOTAL_STEPS })}</p>
        <h1 className="title">{t("onboarding.create.title")}</h1>
        <p className="body muted">{t("onboarding.create.body")}</p>
        <TextField
          label={t("onboarding.create.email")}
          type="email"
          autoComplete="email"
          required
          value={fields.email}
          onChange={(event) => {
            setFields({ ...fields, email: event.target.value });
          }}
        />
        <TextField
          label={t("onboarding.create.password")}
          hint={t("onboarding.create.passwordHint")}
          type="password"
          autoComplete="new-password"
          required
          value={fields.password}
          onChange={(event) => {
            setFields({ ...fields, password: event.target.value });
          }}
        />
        <TextField
          label={t("onboarding.create.confirm")}
          type="password"
          autoComplete="new-password"
          required
          aria-invalid={mismatch}
          value={fields.confirm}
          onChange={(event) => {
            setFields({ ...fields, confirm: event.target.value });
          }}
        />
        {mismatch && (
          <p className="alert" role="alert">
            {t("onboarding.create.mismatch")}
          </p>
        )}
        <ErrorAlert error={error} />
        <p className="status" aria-live="polite">
          {busy ? t("onboarding.create.working") : ""}
        </p>
        <HoloButton type="submit" disabled={busy}>
          {t("onboarding.create.submit")}
        </HoloButton>
        <p className="footnote muted">{t("app.preAudit")}</p>
      </form>
    </main>
  );
}
