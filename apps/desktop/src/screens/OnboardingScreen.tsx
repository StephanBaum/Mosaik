/**
 * Onboarding (UX_GUIDELINES §8): create the account, then the Emergency Kit with its
 * verification step. Import and the extension (steps 3 to 5) belong to the onboarding
 * track; the Emergency Kit PDF too.
 */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { TextField } from "../components/TextField.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { CreatedAccount, IpcError, SessionState } from "../ipc/contract/index.ts";

const TOTAL_STEPS = 5;

export function OnboardingScreen({ onDone }: { onDone: (session: SessionState) => void }) {
  const [created, setCreated] = useState<CreatedAccount | null>(null);
  return created === null ? (
    <CreateAccountStep onCreated={setCreated} />
  ) : (
    <EmergencyKitStep
      created={created}
      onVerified={() => {
        onDone(created.session);
      }}
    />
  );
}

function CreateAccountStep({ onCreated }: { onCreated: (created: CreatedAccount) => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [mismatch, setMismatch] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setError(null);
    if (password !== confirm) {
      setMismatch(true);
      return;
    }
    setMismatch(false);
    setBusy(true);
    const result = await ipc.call("create_account", {
      email,
      password,
      defaultVaultName: t("vault.defaultName"),
    });
    setBusy(false);
    if (result.ok) {
      setPassword("");
      setConfirm("");
      onCreated(result.value);
    } else {
      setError(result.error);
    }
  }

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
          value={email}
          onChange={(event) => {
            setEmail(event.target.value);
          }}
        />
        <TextField
          label={t("onboarding.create.password")}
          hint={t("onboarding.create.passwordHint")}
          type="password"
          autoComplete="new-password"
          required
          value={password}
          onChange={(event) => {
            setPassword(event.target.value);
          }}
        />
        <TextField
          label={t("onboarding.create.confirm")}
          type="password"
          autoComplete="new-password"
          required
          aria-invalid={mismatch}
          value={confirm}
          onChange={(event) => {
            setConfirm(event.target.value);
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
        <Button variant="primary" type="submit" disabled={busy}>
          {t("onboarding.create.submit")}
        </Button>
        <p className="footnote muted">{t("app.preAudit")}</p>
      </form>
    </main>
  );
}

function EmergencyKitStep({
  created,
  onVerified,
}: {
  created: CreatedAccount;
  onVerified: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [lastGroup, setLastGroup] = useState("");
  const [mistyped, setMistyped] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const kit = created.emergencyKit;

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    const result = await ipc.call("verify_emergency_kit", { lastGroup });
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
        <p className="eyebrow">{t("onboarding.step", { current: 2, total: TOTAL_STEPS })}</p>
        <h1 className="title">{t("onboarding.secretKey.title")}</h1>
        <p className="body muted">{t("onboarding.secretKey.body")}</p>
        <dl className="kit">
          <dt>{t("onboarding.create.email")}</dt>
          <dd>{kit.email}</dd>
          <dt>{t("onboarding.kit.accountId")}</dt>
          <dd className="mono">{kit.accountId}</dd>
          <dt>{t("onboarding.kit.secretKey")}</dt>
          <dd className="mono secret-key">{kit.secretKey}</dd>
        </dl>
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
        <Button variant="primary" type="submit">
          {t("onboarding.kit.confirm")}
        </Button>
      </form>
    </main>
  );
}
