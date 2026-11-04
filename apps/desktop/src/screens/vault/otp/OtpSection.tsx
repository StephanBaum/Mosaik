/**
 * The "One-time code (2FA)" section of the item editor (ADR-0031). It shows the sources
 * (scan the screen, paste a screenshot, choose an image, type the key), opens the setup
 * dialog, and reports what the editor should send for `otp` as an `OtpFormValue`.
 */
import { useEffect, useState } from "react";
import { Button } from "../../../components/Button.tsx";
import { FieldError } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type {
  ItemKind,
  OtpPending,
  OtpScheme,
  OtpScanSource,
  OtpSummary,
  VaultSummary,
} from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import { OtpSetupDialog } from "./OtpSetupDialog.tsx";
import { OtpSources } from "./OtpSources.tsx";

export type OtpStart = { kind: "scan"; source: OtpScanSource } | { kind: "manual" };

export interface OtpSetupResult {
  /** The preview, for the summary row. */
  pending: OtpPending;
  /** As edited on the confirmation screen. */
  issuer: string | null;
  accountName: string | null;
  target: { kind: "item" } | { kind: "hardwareVault"; vaultId: string };
}

/** What the editor will send for `otp`. */
export type OtpFormValue =
  | { kind: "unchanged" }
  | { kind: "remove" }
  | { kind: "pending"; result: OtpSetupResult };

export const OTP_UNCHANGED: OtpFormValue = { kind: "unchanged" };

interface Described {
  issuer: string | null;
  scheme: OtpScheme;
  digits: number;
  period: number | null;
}

function Summary({ code, note }: { code: Described; note?: string | undefined }) {
  const { t } = useI18n();
  return (
    <div className="otp-section__summary">
      <span className="otp-section__issuer">{code.issuer ?? t("otp.summary.noIssuer")}</span>
      <span className="field__hint">
        {t("otp.confirm.details", {
          scheme: code.scheme,
          digits: code.digits,
          period: code.period ?? 0,
        })}
      </span>
      {note !== undefined && <span className="field__hint">{note}</span>}
    </div>
  );
}

/** Whether this system can scan the screen; unknown (hidden) until `app_status` answers. */
function useScreenScan(): boolean {
  const ipc = useIpc();
  const [supported, setSupported] = useState(false);
  useEffect(() => {
    let current = true;
    void ipc.call("app_status", null).then((result) => {
      if (current && result.ok) {
        setSupported(result.value.otpScreenScan);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc]);
  return supported;
}

function discardPending(ipc: ReturnType<typeof useIpc>, value: OtpFormValue): void {
  if (value.kind === "pending") {
    void ipc.call("otp_pending_discard", { pendingId: value.result.pending.pendingId });
  }
}

interface BodyProps {
  value: OtpFormValue;
  existing: OtpSummary | null;
  showSources: boolean;
  screenScan: boolean;
  onPick: (start: OtpStart) => void;
  onReplace: () => void;
  onKeepExisting: () => void;
  onChange: (value: OtpFormValue) => void;
  onCancelPending: () => void;
}

/** What the section shows for the current value: sources, the new code, or the saved one. */
function SectionBody(props: BodyProps) {
  const { t } = useI18n();
  const { value, existing, onChange } = props;
  if (value.kind === "pending") {
    return (
      <PendingRow
        result={value.result}
        onReplace={props.onReplace}
        onCancel={props.onCancelPending}
      />
    );
  }
  if (value.kind === "remove") {
    return (
      <div className="otp-section__row">
        <span>{t("otp.summary.removed")}</span>
        <Button
          size="sm"
          onClick={() => {
            onChange(OTP_UNCHANGED);
          }}
        >
          {t("otp.action.keep")}
        </Button>
      </div>
    );
  }
  if (props.showSources) {
    return (
      <>
        <OtpSources screenScan={props.screenScan} onPick={props.onPick} />
        {existing !== null && (
          <Button size="sm" variant="ghost" onClick={props.onKeepExisting}>
            {t("otp.action.keep")}
          </Button>
        )}
      </>
    );
  }
  return existing === null ? null : (
    <ExistingRow
      existing={existing}
      onReplace={props.onReplace}
      onRemove={() => {
        onChange({ kind: "remove" });
      }}
    />
  );
}

export function OtpSection({
  kind,
  existing,
  value,
  onChange,
  vaults,
  prefill,
  missing = false,
}: {
  kind: ItemKind;
  /** The code the item already has. */
  existing: OtpSummary | null;
  value: OtpFormValue;
  onChange: (value: OtpFormValue) => void;
  vaults: readonly VaultSummary[];
  prefill: { issuer: string; account: string };
  /** A one-time code item must have a code: show why saving is blocked. */
  missing?: boolean;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const screenScan = useScreenScan();
  const [start, setStart] = useState<OtpStart | null>(null);
  const [replacing, setReplacing] = useState(false);

  if (kind !== "login" && kind !== "totp") {
    return null;
  }
  const showSources = replacing || (value.kind === "unchanged" && existing === null);

  return (
    <section className="otp-section" aria-labelledby="otp-section-label">
      <h3 id="otp-section-label" className="field__label">
        {t("otp.section.label")}
      </h3>
      {showSources && existing === null && <p className="field__hint">{t("otp.section.empty")}</p>}
      <SectionBody
        value={value}
        existing={existing}
        showSources={showSources}
        screenScan={screenScan}
        onPick={setStart}
        onReplace={() => {
          discardPending(ipc, value);
          onChange(OTP_UNCHANGED);
          setReplacing(true);
        }}
        onKeepExisting={() => {
          setReplacing(false);
        }}
        onChange={onChange}
        onCancelPending={() => {
          discardPending(ipc, value);
          onChange(OTP_UNCHANGED);
        }}
      />
      {missing && <FieldError id="otp-section-missing" message={t("otp.section.required")} />}
      {start !== null && (
        <OtpSetupDialog
          start={start}
          prefill={prefill}
          vaults={vaults}
          itemKind={kind}
          screenScan={screenScan}
          onClose={() => {
            setStart(null);
          }}
          onDone={(result) => {
            setStart(null);
            setReplacing(false);
            onChange({ kind: "pending", result });
          }}
        />
      )}
    </section>
  );
}

function PendingRow({
  result,
  onReplace,
  onCancel,
}: {
  result: OtpSetupResult;
  onReplace: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const { pending } = result;
  return (
    <div className="otp-section__row">
      <Summary
        code={{ ...pending, issuer: result.issuer }}
        note={t("otp.summary.pending")}
      />
      <div className="otp-section__actions">
        <Button size="sm" onClick={onReplace}>
          {t("otp.action.replace")}
        </Button>
        <Button size="sm" variant="ghost" onClick={onCancel}>
          {t("otp.action.cancelSetup")}
        </Button>
      </div>
    </div>
  );
}

function ExistingRow({
  existing,
  onReplace,
  onRemove,
}: {
  existing: OtpSummary;
  onReplace: () => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="otp-section__row">
      <Summary
        code={existing}
        note={existing.hardwareProtected ? t("vault.hardware.eyebrow") : undefined}
      />
      <div className="otp-section__actions">
        <Button size="sm" onClick={onReplace}>
          {t("otp.action.replace")}
        </Button>
        <Button size="sm" variant="ghost" onClick={onRemove}>
          {t("otp.action.remove")}
        </Button>
      </div>
    </div>
  );
}
