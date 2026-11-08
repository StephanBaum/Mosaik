/**
 * Settings, "Sharing": this account's own safety code (to read to the people it shares with) and
 * the people looked up so far with whether their code was compared. Sharing needs the Mosaic
 * server; with a sync folder (or without sync) the section says why it is off.
 */
import { useCallback, useEffect, useId, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Chip } from "../../components/Chip.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { ContactView, IpcError, SharingIdentity } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { SettingsSection } from "../SettingsSection.tsx";
import { useBackendKind } from "./hooks.ts";
import { NotHostedNote } from "./NotHostedNote.tsx";
import { CodeDisplay } from "./SafetyCodePanel.tsx";
import { VerifyContactDialog } from "./VerifyContactDialog.tsx";

function OwnCode({ identity }: { identity: SharingIdentity }) {
  const { t } = useI18n();
  const labelId = useId();
  return (
    <div className="stack">
      <h3 id={labelId} className="subtitle">
        {t("sharing.ownCode.title")}
      </h3>
      {identity.safetyCode === null ? (
        <p className="body muted">{t("sharing.ownCode.pending")}</p>
      ) : (
        <>
          <CodeDisplay code={identity.safetyCode} labelledBy={labelId} />
          <p className="field__hint">{t("sharing.ownCode.body")}</p>
        </>
      )}
    </div>
  );
}

function ContactRow({ contact, onCompare }: { contact: ContactView; onCompare: () => void }) {
  const { t } = useI18n();
  const status = contact.changed
    ? t("sharing.contacts.changed")
    : contact.verified
      ? t("sharing.contacts.verified")
      : t("sharing.contacts.notVerified");
  return (
    <li className="row">
      <div className="row__main">
        <span className="row__value">
          <UserText>{contact.email}</UserText>
        </span>
        <span className="row__meta">
          <Chip tone={contact.verified && !contact.changed ? "neutral" : "warning"}>{status}</Chip>
        </span>
      </div>
      {(!contact.verified || contact.changed) && (
        <Button size="sm" onClick={onCompare} aria-label={`${t("sharing.contacts.verify")} ${contact.email}`}>
          {contact.changed ? t("sharing.contacts.review") : t("sharing.contacts.verify")}
        </Button>
      )}
    </li>
  );
}

export function SharingSettings() {
  const { t } = useI18n();
  const ipc = useIpc();
  const { kind } = useBackendKind();
  const [identity, setIdentity] = useState<SharingIdentity | null>(null);
  const [contacts, setContacts] = useState<ContactView[]>([]);
  const [error, setError] = useState<IpcError | null>(null);
  const [verifying, setVerifying] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const reload = useCallback(() => {
    setVersion((n) => n + 1);
  }, []);

  useEffect(() => {
    if (kind !== "hosted") {
      return undefined;
    }
    const live = { current: true };
    void (async () => {
      const own = await ipc.call("sharing_identity", null);
      const list = await ipc.call("sharing_contacts", null);
      if (!live.current) {
        return;
      }
      setIdentity(own.ok ? own.value : null);
      setError(own.ok ? null : own.error);
      setContacts(list.ok ? list.value.contacts : []);
    })();
    return () => {
      live.current = false;
    };
  }, [ipc, kind, version]);

  return (
    <SettingsSection title={t("sharing.title")} body={t("sharing.body")}>
      {(kind === "folder" || kind === "none") && <NotHostedNote kind={kind} />}
      <ErrorAlert error={error} />
      {kind === "hosted" && identity !== null && (
        <>
          <OwnCode identity={identity} />
          <h3 className="subtitle">{t("sharing.contacts.title")}</h3>
          {contacts.length === 0 ? (
            <p className="body muted">{t("sharing.contacts.empty")}</p>
          ) : (
            <ul className="rows">
              {contacts.map((contact) => (
                <ContactRow
                  key={contact.accountId}
                  contact={contact}
                  onCompare={() => {
                    setVerifying(contact.email);
                  }}
                />
              ))}
            </ul>
          )}
        </>
      )}
      {verifying !== null && (
        <VerifyContactDialog
          email={verifying}
          onVerified={reload}
          onClose={() => {
            setVerifying(null);
          }}
        />
      )}
    </SettingsSection>
  );
}
