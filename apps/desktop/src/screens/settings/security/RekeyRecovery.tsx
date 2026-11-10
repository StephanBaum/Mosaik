/**
 * Another device renewed the account's keys (a Secret Key rotation or a re-key). There is no
 * email: this notice is how the user learns of it. The way back is the new Secret Key from the
 * new Emergency Kit and the master password. A Secret Key this device used before is refused
 * with its own message; nothing is adopted without the new key.
 */
import { useEffect, useRef, useState, type SyntheticEvent } from "react";
import { Button } from "../../../components/Button.tsx";
import { Callout } from "../../../components/Callout.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { HoloButton } from "../../../components/HoloButton.tsx";
import { PasswordField } from "../../../components/PasswordField.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useToast } from "../../../components/ToastProvider.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { IpcError, RekeyMarkerView, SyncOverview } from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import { useSyncOverview } from "../../sync/hooks.ts";
import { focusLost, focusMain, needsSecondFactor } from "./hooks.ts";
import { RekeyMarkerList } from "./RekeyMarkers.tsx";

/** The form: the master password, the new Secret Key and, when needed, a code. */
// eslint-disable-next-line max-lines-per-function -- a three-field form.
export function RekeyRecoveryForm({
  needsCode,
  onRecovered,
  onCancel,
}: {
  needsCode: boolean;
  onRecovered: (overview: SyncOverview) => void;
  /** Absent where the form is part of the page, not a dialog. */
  onCancel?: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [password, setPassword] = useState("");
  const [secretKey, setSecretKey] = useState("");
  const [code, setCode] = useState("");
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_recover_password", {
      newPassword: password,
      secretKey,
      code: needsCode && code !== "" ? code : null,
    });
    setBusy(false);
    setPassword("");
    setSecretKey("");
    setCode("");
    if (result.ok) {
      onRecovered(result.value);
    } else {
      setError(result.error);
    }
  }

  return (
    <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
      <p className="body muted">{t("rotation.recovery.body")}</p>
      <Callout tone="warning">{t("rotation.recovery.warning")}</Callout>
      <PasswordField
        toggleLabel="password"
        label={t("rotation.recovery.password")}
        autoComplete="current-password"
        data-autofocus=""
        value={password}
        onChange={(event) => {
          setPassword(event.target.value);
        }}
      />
      <TextField
        label={t("rotation.recovery.secretKey")}
        mono
        autoCapitalize="characters"
        autoComplete="off"
        spellCheck={false}
        value={secretKey}
        onChange={(event) => {
          setSecretKey(event.target.value);
        }}
      />
      {needsCode && (
        <TextField
          label={t("rotation.credentials.code")}
          inputMode="numeric"
          autoComplete="one-time-code"
          mono
          value={code}
          onChange={(event) => {
            setCode(event.target.value);
          }}
        />
      )}
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        {onCancel !== undefined && <Button onClick={onCancel}>{t("action.cancel")}</Button>}
        <HoloButton type="submit" disabled={busy || password === "" || secretKey === ""}>
          {t("rotation.recovery.submit")}
        </HoloButton>
      </div>
    </form>
  );
}

/** The dialog around the form. */
export function RekeyRecoveryDialog({
  needsCode,
  onRecovered,
  onClose,
}: {
  needsCode: boolean;
  onRecovered: (overview: SyncOverview) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <Dialog title={t("rotation.recovery.title")} onClose={onClose}>
      <RekeyRecoveryForm needsCode={needsCode} onRecovered={onRecovered} onCancel={onClose} />
    </Dialog>
  );
}

/** The unconfirmed notices while sync is paused, and where focus goes when one is ignored. */
function useMarkers(pending: boolean) {
  const ipc = useIpc();
  const [markers, setMarkers] = useState<RekeyMarkerView[]>([]);
  const region = useRef<HTMLDivElement>(null);
  const refocus = useRef(false);
  // After a notice was ignored the control that had focus is gone: the region (or, when the whole
  // notice went away, the page) takes it.
  useEffect(() => {
    if (refocus.current && focusLost()) {
      refocus.current = false;
      if (region.current === null) {
        focusMain();
      } else {
        region.current.focus();
      }
    }
  }, [markers, pending]);
  useEffect(() => {
    if (!pending) {
      return undefined;
    }
    let current = true;
    void ipc.call("sync_rekey_markers", null).then((result) => {
      if (current && result.ok) {
        setMarkers(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, pending]);
  const dismissed = (marker: RekeyMarkerView) => {
    refocus.current = true;
    setMarkers((all) => all.filter((m) => m.sha256 !== marker.sha256));
  };
  return { markers: pending ? markers : [], region, dismissed };
}

/** The in-app notice: sync is paused because a verified marker says the keys were renewed elsewhere. */
export function RekeyNotice() {
  const { t } = useI18n();
  const toast = useToast();
  const { overview, setOverview, refresh } = useSyncOverview();
  const [open, setOpen] = useState(false);
  const pending = overview?.rekeyPending ?? false;
  const { markers, region, dismissed } = useMarkers(pending);
  // The title is announced from a status region that stays mounted: one that appears with its
  // text is often not read.
  const status = (
    <p className="visually-hidden" role="status">
      {pending ? t("rotation.recovery.title") : ""}
    </p>
  );
  if (overview === null || !overview.rekeyPending) {
    return status;
  }
  return (
    <div role="region" aria-label={t("rotation.recovery.title")} ref={region} tabIndex={-1}>
      {status}
      <Callout tone="warning">
        <p className="body" aria-hidden="true">
          <strong>{t("rotation.recovery.title")}</strong>
        </p>
        <p className="body">{t("rotation.recovery.notice")}</p>
        <div className="actions">
          <Button
            onClick={() => {
              setOpen(true);
            }}
          >
            {t("rotation.recovery.open")}
          </Button>
        </div>
        <RekeyMarkerList
          markers={markers}
          onDismissed={(marker, next) => {
            dismissed(marker);
            setOverview(next);
            refresh();
          }}
        />
      </Callout>
      {open && (
        <RekeyRecoveryDialog
          needsCode={needsSecondFactor(overview)}
          onRecovered={(next) => {
            setOverview(next);
            refresh();
            setOpen(false);
            toast(t("rotation.recovery.done"));
          }}
          onClose={() => {
            setOpen(false);
          }}
        />
      )}
    </div>
  );
}
