/**
 * The sharing dialog of one vault (opened from the top bar): the members, their roles and the
 * way to invite someone. Sharing needs the Mosaic server, so a sync folder (or no sync) shows
 * why instead.
 */
import { useState } from "react";
import { Dialog } from "../../components/Dialog.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { VaultSummary } from "../../ipc/contract/index.ts";
import { vaultLabel } from "../vault/Sidebar.tsx";
import { useBackendKind } from "./hooks.ts";
import { MembersPanel } from "./MembersPanel.tsx";
import { NotHostedNote } from "./NotHostedNote.tsx";
import { ShareVaultDialog } from "./ShareVaultDialog.tsx";
import { VerifyContactDialog } from "./VerifyContactDialog.tsx";
import { isolate } from "../sync/hooks.ts";

export function VaultSharingDialog({
  vault,
  onChanged,
  onClose,
}: {
  vault: VaultSummary;
  /** Members or roles changed (sharing state, not vault items). */
  onChanged: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const { kind } = useBackendKind();
  const [inviting, setInviting] = useState(false);
  const [verifying, setVerifying] = useState<string | null>(null);
  const [version, setVersion] = useState(0);
  const name = vaultLabel(vault, (key) => t(key));
  // The dialogs for inviting and verifying sit next to this one, not inside it.
  return (
    <>
      <Dialog
      title={t("sharing.share.dialog", { vault: isolate(name) })}
      size="wide"
      closeButton
      onClose={onClose}
    >
      <div className="stack">
        {kind === "folder" || kind === "none" ? <NotHostedNote kind={kind} /> : null}
        {kind === "hosted" && (
          <MembersPanel
            vaultId={vault.vaultId}
            version={version}
            onInvite={() => {
              setInviting(true);
            }}
            onVerify={setVerifying}
            onLeft={() => {
              onChanged();
              onClose();
            }}
            onChanged={onChanged}
          />
        )}
      </div>
      </Dialog>
      {inviting && (
        <ShareVaultDialog
          vault={vault}
          onShared={() => {
            setVersion((n) => n + 1);
            onChanged();
          }}
          onClose={() => {
            setInviting(false);
          }}
        />
      )}
      {verifying !== null && (
        <VerifyContactDialog
          email={verifying}
          onVerified={() => {
            setVersion((n) => n + 1);
          }}
          onClose={() => {
            setVerifying(null);
          }}
        />
      )}
    </>
  );
}
