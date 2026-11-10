/**
 * The passwords of a change: the current master password, for a password change or a re-key the
 * new one twice, for a protection upgrade the level, and the second-factor code when the sync
 * account has one. The values live in the wizard; they are cleared as soon as they are used.
 */
import type { SyntheticEvent } from "react";
import { Button } from "../../../components/Button.tsx";
import { Callout } from "../../../components/Callout.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { HoloButton } from "../../../components/HoloButton.tsx";
import { PasswordField } from "../../../components/PasswordField.tsx";
import { Select } from "../../../components/Select.tsx";
import { Switch } from "../../../components/Switch.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import { rotationProfiles, type IpcError, type RotationProfile } from "../../../ipc/contract/index.ts";
import { StepHeading } from "../../sharing/StepHeading.tsx";

export type WizardKind = "password" | "secretKey" | "profile" | "rekey";

export interface Credentials {
  password: string;
  newPassword: string;
  confirm: string;
  code: string;
  profile: RotationProfile;
  force: boolean;
}

export const emptyCredentials: Credentials = {
  password: "",
  newPassword: "",
  confirm: "",
  code: "",
  profile: "strong",
  force: false,
};

/** Whether the form has what the step needs, and the first thing that is wrong with it. */
export function credentialsProblem(
  kind: WizardKind,
  values: Credentials,
  needsCode: boolean,
): "mismatch" | "same" | "incomplete" | null {
  if (values.password === "" || (needsCode && values.code === "")) {
    return "incomplete";
  }
  if (kind === "password" || kind === "rekey") {
    if (values.newPassword === "" || values.confirm === "") {
      return "incomplete";
    }
    if (values.newPassword !== values.confirm) {
      return "mismatch";
    }
    if (values.newPassword === values.password) {
      return "same";
    }
  }
  return null;
}

// eslint-disable-next-line max-lines-per-function, complexity -- one form whose fields depend on the kind of change.
export function CredentialsStep({
  kind,
  values,
  needsCode,
  slow,
  busy,
  error,
  submit,
  danger = false,
  onChange,
  onSubmit,
  onBack,
}: {
  kind: WizardKind;
  values: Credentials;
  needsCode: boolean;
  /** `profileTooSlow` came back: offer to go on anyway. */
  slow: boolean;
  busy: boolean;
  error: IpcError | null;
  /** The label of the submit button (a verb where this step changes the account). */
  submit: string;
  /** The submit button is a destructive verb button, not the primary action. */
  danger?: boolean;
  onChange: (next: Credentials) => void;
  onSubmit: (event: SyntheticEvent) => void;
  onBack: () => void;
}) {
  const { t } = useI18n();
  const problem = credentialsProblem(kind, values, needsCode);
  const twoPasswords = kind === "password" || kind === "rekey";
  const canSubmit = !busy && problem === null && !(slow && !values.force && kind === "profile");
  const set = (patch: Partial<Credentials>) => {
    onChange({ ...values, ...patch });
  };
  return (
    <form className="stack" onSubmit={onSubmit} noValidate>
      <StepHeading>
        {twoPasswords ? t("rotation.credentials.password.title") : t("rotation.credentials.title")}
      </StepHeading>
      <PasswordField
        toggleLabel="password"
        label={t("rotation.credentials.current")}
        autoComplete="current-password"
        data-autofocus=""
        value={values.password}
        onChange={(event) => {
          set({ password: event.target.value });
        }}
      />
      {twoPasswords && (
        <>
          <PasswordField
            toggleLabel="password"
            label={t("rotation.credentials.new")}
            hint={t("rotation.credentials.hint")}
            autoComplete="new-password"
            value={values.newPassword}
            onChange={(event) => {
              set({ newPassword: event.target.value });
            }}
          />
          <PasswordField
            toggleLabel="password"
            label={t("rotation.credentials.confirm")}
            autoComplete="new-password"
            error={
              values.confirm !== "" && problem === "mismatch"
                ? t("rotation.credentials.mismatch")
                : problem === "same"
                  ? t("rotation.credentials.same")
                  : undefined
            }
            value={values.confirm}
            onChange={(event) => {
              set({ confirm: event.target.value });
            }}
          />
        </>
      )}
      {kind === "profile" && (
        <>
          <Select
            label={t("rotation.credentials.profile")}
            value={values.profile}
            options={rotationProfiles.map((profile) => ({
              value: profile,
              label: t(`rotation.credentials.profile.${profile}`),
            }))}
            onChange={(event) => {
              set({
                profile: rotationProfiles.find((x) => x === event.target.value) ?? "strong",
                force: false,
              });
            }}
          />
          {slow && (
            <div className="stack">
              <Callout tone="warning" role="alert">
                {t("rotation.credentials.profile.slow")}
              </Callout>
              <Switch
                label={t("rotation.credentials.profile.force")}
                checked={values.force}
                onChange={(event) => {
                  set({ force: event.target.checked });
                }}
              />
            </div>
          )}
        </>
      )}
      {needsCode && (
        <TextField
          label={t("rotation.credentials.code")}
          inputMode="numeric"
          autoComplete="one-time-code"
          mono
          value={values.code}
          onChange={(event) => {
            set({ code: event.target.value });
          }}
        />
      )}
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button variant="ghost" onClick={onBack}>
          {t("rotation.back")}
        </Button>
        {danger ? (
          <Button type="submit" variant="dangerSolid" disabled={!canSubmit}>
            {submit}
          </Button>
        ) : (
          <HoloButton type="submit" disabled={!canSubmit}>
            {submit}
          </HoloButton>
        )}
      </div>
    </form>
  );
}
