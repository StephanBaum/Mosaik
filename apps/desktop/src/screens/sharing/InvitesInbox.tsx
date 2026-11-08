/**
 * The invites inbox (sidebar entry "Invites", with a count): the invites sent to this account,
 * each reviewed in a dialog before anything is joined, and the invites this account sent that
 * still wait for an answer. Read again whenever Rust reports `sharingChanged`.
 */
import { useCallback, useEffect, useState } from "react";
import { useEvents } from "../../app/EventsProvider.tsx";
import { Button } from "../../components/Button.tsx";
import { Chip } from "../../components/Chip.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type {
  IncomingInviteView,
  InviteList,
  IpcError,
  OutgoingInviteView,
} from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "../sync/hooks.ts";
import { NotHostedNote } from "./NotHostedNote.tsx";
import { useBackendKind, useFormatWhen, useRoleLabel, type BackendKind } from "./hooks.ts";
import { InvitePreviewDialog } from "./InvitePreviewDialog.tsx";

/** The invites of both directions, read again on every `sharingChanged` and on `reload`. */
function useInviteList(kind: BackendKind) {
  const ipc = useIpc();
  const { sharing } = useEvents();
  const [list, setList] = useState<InviteList | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [local, setLocal] = useState(0);
  useEffect(() => {
    if (kind !== "hosted") {
      return undefined;
    }
    const live = { current: true };
    void ipc.call("invites_list", null).then((result) => {
      if (!live.current) {
        return;
      }
      setList(result.ok ? result.value : null);
      setError(result.ok ? null : result.error);
    });
    return () => {
      live.current = false;
    };
  }, [ipc, kind, sharing.version, local]);
  const reload = useCallback(() => {
    setLocal((n) => n + 1);
  }, []);
  return { list, error, setError, reload };
}

function IncomingList({
  invites,
  onReview,
}: {
  invites: readonly IncomingInviteView[];
  onReview: (invite: IncomingInviteView) => void;
}) {
  const { t } = useI18n();
  const roleLabel = useRoleLabel();
  const formatWhen = useFormatWhen();
  if (invites.length === 0) {
    return <p className="body muted">{t("sharing.inbox.empty")}</p>;
  }
  return (
    <ul className="rows" aria-label={t("sharing.inbox.title")}>
      {invites.map((invite) => (
        <li key={invite.inviteId} className="row">
          <div className="row__main">
            <span className="row__value">{t("sharing.inbox.from", { email: isolate(invite.senderEmail) })}</span>
            <span className="row__meta">
              <Chip>{roleLabel(invite.role)}</Chip>
              <span>{t("sharing.invite.expires", { when: formatWhen(invite.expiresAt) })}</span>
            </span>
          </div>
          <Button
            aria-label={`${t("sharing.inbox.review")}: ${invite.senderEmail}`}
            onClick={() => {
              onReview(invite);
            }}
          >
            {t("sharing.inbox.review")}
          </Button>
        </li>
      ))}
    </ul>
  );
}

function OutgoingList({
  invites,
  onWithdraw,
}: {
  invites: readonly OutgoingInviteView[];
  onWithdraw: (inviteId: string) => void;
}) {
  const { t } = useI18n();
  const roleLabel = useRoleLabel();
  const formatWhen = useFormatWhen();
  if (invites.length === 0) {
    return <p className="body muted">{t("sharing.inbox.outgoing.empty")}</p>;
  }
  return (
    <ul className="rows">
      {invites.map((invite) => (
        <li key={invite.inviteId} className="row">
          <div className="row__main">
            <span className="row__value">
              {invite.recipientEmail === null ? (
                t("sharing.members.unknownEmail")
              ) : (
                <UserText>{invite.recipientEmail}</UserText>
              )}
            </span>
            <span className="row__meta">
              <UserText>{invite.vaultName ?? t("sharing.inbox.unknownVault")}</UserText>
              <Chip>{roleLabel(invite.role)}</Chip>
              <span>{t("sharing.invite.expires", { when: formatWhen(invite.expiresAt) })}</span>
            </span>
          </div>
          <Button
            variant="ghost"
            aria-label={`${t("sharing.invite.cancel")}: ${invite.recipientEmail ?? ""}`}
            onClick={() => {
              onWithdraw(invite.inviteId);
            }}
          >
            {t("sharing.invite.cancel")}
          </Button>
        </li>
      ))}
    </ul>
  );
}

export function InvitesInbox() {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const { kind } = useBackendKind();
  const { list, error, setError, reload } = useInviteList(kind);
  const [reviewing, setReviewing] = useState<IncomingInviteView | null>(null);

  async function withdraw(inviteId: string) {
    const result = await ipc.call("vault_invite_cancel", { inviteId });
    if (result.ok) {
      toast(t("sharing.invite.cancelled"));
      reload();
    } else {
      setError(result.error);
    }
  }

  return (
    <section className="settings" aria-label={t("sharing.inbox.title")}>
      <div className="settings__intro">
        <h1 className="display">{t("sharing.inbox.title")}</h1>
        <p className="lede">{t("sharing.inbox.intro")}</p>
      </div>
      {(kind === "folder" || kind === "none") && <NotHostedNote kind={kind} />}
      <ErrorAlert error={error} />
      {kind === "hosted" && list !== null && (
        <>
          <IncomingList invites={list.incoming} onReview={setReviewing} />
          <h2 className="subtitle">{t("sharing.inbox.outgoing")}</h2>
          <OutgoingList
            invites={list.outgoing}
            onWithdraw={(inviteId) => {
              void withdraw(inviteId);
            }}
          />
          <div className="actions">
            <Button variant="ghost" onClick={reload}>
              {t("sharing.inbox.refresh")}
            </Button>
          </div>
        </>
      )}
      {reviewing !== null && (
        <InvitePreviewDialog
          invite={reviewing}
          onDone={(outcome) => {
            toast(t(outcome === "joined" ? "sharing.inbox.joined" : "sharing.inbox.declined"));
            reload();
          }}
          onClose={() => {
            setReviewing(null);
          }}
        />
      )}
    </section>
  );
}
