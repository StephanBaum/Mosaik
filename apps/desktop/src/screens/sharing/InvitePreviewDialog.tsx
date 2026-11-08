/**
 * One invite, after Mosaic opened and checked it (CRYPTO_SPEC §8.2): who invites you, to what,
 * as what. Before you can join, the sender's safety code is compared, and when a manager invites
 * you to somebody else's vault the owner's, too: the owner is the one who can change the key.
 * Nothing is stored until "Join vault".
 */
import { useEffect, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IncomingInviteView, InvitePreviewView, IpcError } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "../sync/hooks.ts";
import { useRoleLabel } from "./hooks.ts";
import { SafetyCodePanel } from "./SafetyCodePanel.tsx";

type Phase =
  | { kind: "loading" }
  | { kind: "failed"; error: IpcError }
  | { kind: "ready"; preview: InvitePreviewView };

type Outcome = "joined" | "declined";

function usePreview(inviteId: string): Phase {
  const ipc = useIpc();
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  useEffect(() => {
    const live = { current: true };
    void ipc.call("invite_preview", { inviteId }).then((result) => {
      if (live.current) {
        setPhase(result.ok ? { kind: "ready", preview: result.value } : { kind: "failed", error: result.error });
      }
    });
    return () => {
      live.current = false;
    };
  }, [ipc, inviteId]);
  return phase;
}

/** The answers to an invite: join, decline (also blocking the sender), and verifying the owner. */
function useAnswers(inviteId: string, onDone: (outcome: Outcome) => void, onClose: () => void) {
  const ipc = useIpc();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const answer = async (command: "join" | "decline" | "block", preview: InvitePreviewView | null) => {
    setBusy(true);
    setError(null);
    const result =
      command === "join"
        ? await ipc.call("invite_accept", {
            inviteId,
            safetyCode: preview === null || preview.senderStatus === "verified" ? null : preview.safetyCode,
          })
        : await ipc.call("invite_decline", { inviteId, block: command === "block" });
    setBusy(false);
    if (result.ok) {
      onDone(command === "join" ? "joined" : "declined");
      onClose();
    } else {
      setError(result.error);
    }
  };
  /** Verifies the owner of the vault with the code that was just compared. */
  const verifyOwner = async (preview: InvitePreviewView): Promise<boolean> => {
    setBusy(true);
    setError(null);
    const result = await ipc.call("sharing_verify_contact", {
      accountId: preview.ownerAccountId,
      safetyCode: preview.ownerCode,
    });
    setBusy(false);
    setError(result.ok ? null : result.error);
    return result.ok;
  };
  return { busy, error, answer, verifyOwner };
}

type Answers = ReturnType<typeof useAnswers>;

/** What the user attested to so far: the sender's code, and the owner's (which Rust verified). */
function Verification({
  preview,
  answers,
  onSender,
  onOwner,
  senderOk,
  onClose,
}: {
  preview: InvitePreviewView;
  answers: Answers;
  senderOk: boolean;
  onSender: () => void;
  onOwner: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const { busy, error } = answers;
  if (!senderOk) {
    return (
      <SafetyCodePanel
        email={preview.senderEmail}
        code={preview.safetyCode}
        busy={busy}
        error={error}
        onMatches={onSender}
        onCancel={onClose}
      />
    );
  }
  return (
    <>
      <Callout tone="warning">
        <p className="body">{t("sharing.inbox.verifyOwner", { sender: isolate(preview.senderEmail) })}</p>
      </Callout>
      <SafetyCodePanel
        email={preview.ownerEmail}
        code={preview.ownerCode}
        codeLabel={t("sharing.inbox.ownerCode", { email: isolate(preview.ownerEmail) })}
        busy={busy}
        error={error}
        onMatches={onOwner}
        onCancel={onClose}
      />
    </>
  );
}

function Ready({
  preview,
  answers,
  onClose,
}: {
  preview: InvitePreviewView;
  answers: Answers;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const roleLabel = useRoleLabel();
  const [senderAttested, setSenderAttested] = useState(false);
  const [ownerAttested, setOwnerAttested] = useState(false);
  const senderOk = preview.senderStatus === "verified" || senderAttested;
  const ownerIsSender = preview.ownerAccountId === preview.senderAccountId;
  const ownerOk = ownerIsSender || preview.ownerStatus === "verified" || ownerAttested;
  const { busy, error, answer } = answers;
  return (
    <div className="stack">
      <p className="body">
        {t("sharing.inbox.preview.body", { email: isolate(preview.senderEmail), role: roleLabel(preview.role) })}
      </p>
      {!ownerIsSender && (
        <p className="body muted">{t("sharing.inbox.preview.owner", { email: isolate(preview.ownerEmail) })}</p>
      )}
      <p className="caption">
        <UserText>{preview.vaultName ?? t("sharing.inbox.preview.unnamed")}</UserText>
      </p>
      {senderOk && ownerOk ? (
        <>
          <ErrorAlert error={error} />
          <div className="dialog__actions">
            <Button variant="ghost" disabled={busy} onClick={() => void answer("block", preview)}>
              {t("sharing.inbox.block")}
            </Button>
            <Button disabled={busy} onClick={() => void answer("decline", preview)}>
              {t("sharing.inbox.decline")}
            </Button>
            <HoloButton disabled={busy} onClick={() => void answer("join", preview)}>
              {t("sharing.inbox.accept")}
            </HoloButton>
          </div>
        </>
      ) : (
        <Verification
          preview={preview}
          answers={answers}
          senderOk={senderOk}
          onSender={() => {
            setSenderAttested(true);
          }}
          onOwner={() => {
            void answers.verifyOwner(preview).then(setOwnerAttested);
          }}
          onClose={onClose}
        />
      )}
    </div>
  );
}

function Failed({
  error,
  invite,
  answers,
  onClose,
}: {
  error: IpcError;
  invite: IncomingInviteView;
  answers: Answers;
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="stack">
      {error.code === "sharingInviteUnreadable" ? (
        <p className="alert" role="alert">
          {t("sharing.inbox.unreadable", { email: isolate(invite.senderEmail) })}
        </p>
      ) : (
        <ErrorAlert error={error} />
      )}
      <div className="dialog__actions">
        <Button disabled={answers.busy} onClick={() => void answers.answer("decline", null)}>
          {t("sharing.inbox.decline")}
        </Button>
        <Button onClick={onClose}>{t("action.close")}</Button>
      </div>
    </div>
  );
}

export function InvitePreviewDialog({
  invite,
  onDone,
  onClose,
}: {
  invite: IncomingInviteView;
  /** Joined or declined: the inbox reads again. */
  onDone: (outcome: Outcome) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const phase = usePreview(invite.inviteId);
  const answers = useAnswers(invite.inviteId, onDone, onClose);
  const title =
    phase.kind === "ready"
      ? t("sharing.inbox.preview.title", {
          vault: isolate(phase.preview.vaultName ?? t("sharing.inbox.preview.unnamed")),
        })
      : t("sharing.inbox.title");
  return (
    <Dialog title={title} onClose={onClose}>
      {phase.kind === "loading" && (
        <p className="body muted" role="status">
          {t("sharing.lookup.searching")}
        </p>
      )}
      {phase.kind === "failed" && <Failed error={phase.error} invite={invite} answers={answers} onClose={onClose} />}
      {phase.kind === "ready" && <Ready preview={phase.preview} answers={answers} onClose={onClose} />}
    </Dialog>
  );
}
