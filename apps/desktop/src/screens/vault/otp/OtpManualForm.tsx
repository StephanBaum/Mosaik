/**
 * "Can't scan?": the setup key (or an otpauth:// link) typed or pasted. This is the one
 * path where the secret is in the webview, because the user puts it there (R19); it is
 * cleared from state as soon as the request returns.
 */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../../components/Button.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { HoloButton } from "../../../components/HoloButton.tsx";
import { Select } from "../../../components/Select.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import {
  otpAlgorithms,
  type IpcError,
  type OtpAlgorithm,
  type OtpParseRequest,
} from "../../../ipc/contract/index.ts";

const DIGITS = [6, 7, 8] as const;
const PERIODS = [15, 30, 60, 90] as const;

interface Advanced {
  algorithm: OtpAlgorithm;
  digits: number;
  period: number;
}

const DEFAULT_ADVANCED: Advanced = { algorithm: "sha1", digits: 6, period: 30 };

/** An otpauth:// link carries its own settings; anything else is a bare key. */
function isLink(text: string): boolean {
  return text.trim().toLowerCase().startsWith("otpauth");
}

function request(text: string, issuer: string, account: string, advanced: Advanced): OtpParseRequest {
  if (isLink(text)) {
    return { kind: "uri", uri: text };
  }
  return {
    kind: "key",
    secret: text,
    issuer: issuer.trim() === "" ? null : issuer.trim(),
    accountName: account.trim() === "" ? null : account.trim(),
    ...advanced,
  };
}

function AdvancedFields({
  value,
  onChange,
}: {
  value: Advanced;
  onChange: (patch: Partial<Advanced>) => void;
}) {
  const { t } = useI18n();
  return (
    <details className="otp-setup__advanced">
      <summary>{t("otp.manual.advanced")}</summary>
      <div className="otp-setup__advanced-fields">
        <Select
          label={t("otp.manual.algorithm")}
          value={value.algorithm}
          options={otpAlgorithms.map((each) => ({
            value: each,
            label: t("otp.confirm.algorithmValue", { algorithm: each }),
          }))}
          onChange={(event) => {
            onChange({ algorithm: event.target.value as OtpAlgorithm });
          }}
        />
        <Select
          label={t("otp.manual.digits")}
          value={String(value.digits)}
          options={DIGITS.map((each) => ({ value: String(each), label: String(each) }))}
          onChange={(event) => {
            onChange({ digits: Number(event.target.value) });
          }}
        />
        <Select
          label={t("otp.manual.period")}
          value={String(value.period)}
          options={PERIODS.map((each) => ({
            value: String(each),
            label: t("otp.manual.periodValue", { seconds: each }),
          }))}
          onChange={(event) => {
            onChange({ period: Number(event.target.value) });
          }}
        />
        <p className="field__hint">{t("otp.manual.advancedHint")}</p>
      </div>
    </details>
  );
}

export function OtpManualForm({
  prefill,
  onSubmit,
  onCancel,
}: {
  prefill: { issuer: string; account: string };
  /** Resolves with the error, or `null` when the dialog moved on. */
  onSubmit: (request: OtpParseRequest) => Promise<IpcError | null>;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [text, setText] = useState("");
  const [issuer, setIssuer] = useState(prefill.issuer);
  const [account, setAccount] = useState(prefill.account);
  const [advanced, setAdvanced] = useState(DEFAULT_ADVANCED);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const link = isLink(text);

  async function submit(event: SyntheticEvent) {
    // The editor's own form must not see this submit.
    event.preventDefault();
    event.stopPropagation();
    setBusy(true);
    setError(null);
    const sent = request(text, issuer, account, advanced);
    const failed = await onSubmit(sent);
    setText("");
    setBusy(false);
    setError(failed);
  }

  return (
    <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
      <h3 className="otp-setup__heading">{t("otp.manual.title")}</h3>
      <TextField
        label={t("otp.manual.input")}
        hint={t("otp.manual.inputHint")}
        mono
        autoFocus
        autoComplete="off"
        spellCheck={false}
        value={text}
        onChange={(event) => {
          setText(event.target.value);
        }}
      />
      {!link && (
        <>
          <TextField
            label={t("otp.manual.issuer")}
            autoComplete="off"
            value={issuer}
            onChange={(event) => {
              setIssuer(event.target.value);
            }}
          />
          <TextField
            label={t("otp.manual.account")}
            autoComplete="off"
            value={account}
            onChange={(event) => {
              setAccount(event.target.value);
            }}
          />
          <AdvancedFields
            value={advanced}
            onChange={(patch) => {
              setAdvanced((current) => ({ ...current, ...patch }));
            }}
          />
        </>
      )}
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onCancel}>{t("action.cancel")}</Button>
        <HoloButton type="submit" disabled={busy || text.trim() === ""}>
          {t("otp.manual.continue")}
        </HoloButton>
      </div>
    </form>
  );
}
