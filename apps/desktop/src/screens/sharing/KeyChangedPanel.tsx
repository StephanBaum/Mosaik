/**
 * A contact's key changed (CRYPTO_SPEC §8.5): the code verified before and the new one, a
 * warning that says what this can mean, and "New code matches". The new code goes through the
 * same comparison as any code (the checkbox, the typed code), and focus starts on "Cancel" so
 * Enter can't accept. Accepting replaces the pin and verifies the new code in one step, after
 * the user compared it with the person. It runs inside `ContactVerifier`, in whatever dialog
 * needs the contact.
 */
import { useId, useState } from "react";
import { Callout } from "../../components/Callout.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { AccountId, IpcError } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "../sync/hooks.ts";
import { CodeDisplay, SafetyCodePanel } from "./SafetyCodePanel.tsx";

export function KeyChangedPanel({
  accountId,
  email,
  oldCode,
  newCode,
  conflicting,
  onAccepted,
  onCancel,
}: {
  accountId: AccountId;
  email: string;
  /** The code that was verified before (`null` when this device never had one). */
  oldCode: string | null;
  newCode: string;
  /** The devices of this account saw different keys for the person. */
  conflicting: boolean;
  onAccepted: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const oldId = useId();
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
    <SafetyCodePanel
      email={email}
      code={newCode}
      codeLabel={t("sharing.keyChanged.new")}
      heading={t("sharing.keyChanged.title", { email: who })}
      intro={t("sharing.code.compareNew", { email: who })}
      matchLabel={t("sharing.keyChanged.accept")}
      focusCancel
      busy={busy}
      error={error}
      onMatches={() => void accept()}
      onCancel={onCancel}
      before={
        <>
          <Callout tone="warning">
            <p className="body safety__text">{t("sharing.keyChanged.body", { email: who })}</p>
          </Callout>
          {conflicting && (
            <Callout tone="warning">
              <p className="body safety__text">{t("sharing.code.conflicting", { email: who })}</p>
            </Callout>
          )}
          {oldCode !== null && (
            <div className="safety__side">
              <p id={oldId} className="row__label">
                {t("sharing.keyChanged.old")}
              </p>
              <CodeDisplay code={oldCode} labelledBy={oldId} />
            </div>
          )}
        </>
      }
    />
  );
}
