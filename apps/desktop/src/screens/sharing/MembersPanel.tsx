/**
 * Who can open a vault: the members with their roles, the people not verified yet, the invites
 * that wait, and what is going on behind a removal. Rust asks removing and leaving in a native
 * dialog with texts it owns. The owner's device changes the vault key; a manager's removal waits
 * for it, and the panel says so.
 */
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type {
  MemberRow,
  OutgoingInviteView,
  VaultId,
  VaultMembersView,
} from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { MemberItem } from "./MemberItem.tsx";
import { useMemberHandlers, type RunChange } from "./useMemberHandlers.ts";
import { useChange, useMembersData } from "./useMembers.ts";

/** The notices under the list: a removal that waits for the owner, files the key change missed. */
function Notices({ view }: { view: VaultMembersView }) {
  const { t } = useI18n();
  return (
    <>
      {view.pendingRotation && (
        <Callout tone="warning">
          <p className="body">{t("sharing.rotation.running")}</p>
        </Callout>
      )}
      {view.unprotectedAttachments > 0 && (
        <Callout tone="warning">
          <p className="body">{t("sharing.rotation.unprotected", { count: view.unprotectedAttachments })}</p>
        </Callout>
      )}
      {view.ownRole === "manage" && <p className="field__hint">{t("sharing.members.managerNote")}</p>}
      {(view.ownRole === "write" || view.ownRole === "read") && (
        <p className="field__hint">{t("sharing.members.readerNote")}</p>
      )}
    </>
  );
}

function LeaveButton({
  vaultId,
  busy,
  run,
  onLeft,
}: {
  vaultId: VaultId;
  busy: boolean;
  run: RunChange;
  onLeft: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  return (
    <Button
      variant="danger"
      disabled={busy}
      onClick={() => {
        void run(() => ipc.call("vault_leave", { vaultId })).then((done) => {
          if (done) {
            toast(t("sharing.members.left"));
            onLeft();
          }
        });
      }}
    >
      {t("sharing.members.leave")}
    </Button>
  );
}

/** The pending invite behind an "invited" row, found by its address. */
function inviteOf(row: MemberRow, invites: readonly OutgoingInviteView[]): OutgoingInviteView | undefined {
  if (row.state !== "invited" || row.email === null) {
    return undefined;
  }
  const email = row.email.toLowerCase();
  return invites.find((i) => i.recipientEmail?.toLowerCase() === email);
}

export function MembersPanel({
  vaultId,
  version,
  onInvite,
  onVerify,
  onLeft,
  onChanged,
}: {
  vaultId: VaultId;
  /** Changes when something else changed the members (an invite was sent). */
  version: number;
  onInvite: () => void;
  /** Opens the safety-code comparison for this address (a dialog next to this one). */
  onVerify: (email: string) => void;
  onLeft: () => void;
  onChanged: () => void;
}) {
  const { t } = useI18n();
  const { loaded, error: loadError, reload } = useMembersData(vaultId, version);
  const { busy, error, run } = useChange(() => {
    reload();
    onChanged();
  });
  const handlersOf = useMemberHandlers(vaultId, run, onVerify);

  if (loadError !== null) {
    return <ErrorAlert error={loadError} />;
  }
  if (loaded === null) {
    return null;
  }
  const { view, invites } = loaded;
  const manager = view.ownRole === "owner" || view.ownRole === "manage";
  return (
    <section className="stack" aria-label={t("sharing.members.title")}>
      <h3 className="subtitle">{t("sharing.members.title")}</h3>
      {view.members.length <= 1 && <p className="body muted">{t("sharing.members.empty")}</p>}
      <ul className="rows">
        {view.members.map((row) => (
          <MemberItem
            key={`${row.accountId}-${row.state}`}
            row={row}
            own={view.ownRole}
            invite={inviteOf(row, invites)}
            busy={busy}
            handlers={handlersOf(row)}
          />
        ))}
      </ul>
      <Notices view={view} />
      <ErrorAlert error={error} />
      <div className="actions">
        {manager && <Button onClick={onInvite}>{t("sharing.members.invite")}</Button>}
        {view.ownRole !== "owner" && <LeaveButton vaultId={vaultId} busy={busy} run={run} onLeft={onLeft} />}
      </div>
    </section>
  );
}
