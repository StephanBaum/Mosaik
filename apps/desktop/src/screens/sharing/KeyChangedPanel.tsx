/**
 * A contact's key changed (CRYPTO_SPEC §8.5): the code verified before and the new one, a
 * warning that says what this can mean, and "Compare and accept". Accepting replaces the pin
 * and verifies the new code in one step, after the user compared it with the person. It runs
 * inside `ContactVerifier`, in whatever dialog needs the contact.
 */
import { useId, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { AccountId, IpcError } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "../sync/hooks.ts";
import { CodeDisplay } from "./SafetyCodePanel.tsx";

export function KeyChangedPanel({
  accountId,
  email,
  oldCode,
  newCode,
  onAccepted,
  onCancel,
}: {
  accountId: AccountId;
  email: string;
  /** The code that was verified before (`null` when this device never had one). */
  oldCode: string | null;
  newCode: string;
  onAccepted: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const oldId = useId();
  const newId = useId();
  const who = isolate(email);

  async function accept() {
    setBusy(true);
    setError(null);
    const request = { accountId, safetyCode: newCode };
    const accepted = await ipc.call("sharing_accept_key_change", request);
    const verified = accepted.ok ? await ipc.call("sharing_verify_contact", request) : accepted;
    setBusy(false);
    if (verified.ok) {
      onAccepted();
    } else {
      setError(verified.error);
    }
  }

  return (
    <section className="stack">
      <h3 className="subtitle">{t("sharing.keyChanged.title", { email: who })}</h3>
      <Callout tone="warning">
        <p className="body">{t("sharing.keyChanged.body", { email: who })}</p>
      </Callout>
      <div className="safety__sides">
        {oldCode !== null && (
          <div className="safety__side">
            <p id={oldId} className="row__label">
              {t("sharing.keyChanged.old")}
            </p>
            <CodeDisplay code={oldCode} labelledBy={oldId} />
          </div>
        )}
        <div className="safety__side">
          <p id={newId} className="row__label">
            {t("sharing.keyChanged.new")}
          </p>
          <CodeDisplay code={newCode} labelledBy={newId} />
        </div>
      </div>
      <p className="body muted">{t("sharing.code.compare", { email: who })}</p>
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onCancel}>{t("action.cancel")}</Button>
        <HoloButton disabled={busy} onClick={() => void accept()}>
          {t("sharing.keyChanged.accept")}
        </HoloButton>
      </div>
    </section>
  );
}
