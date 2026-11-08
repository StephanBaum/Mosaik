/** What a member row can do (change the role, remove, withdraw the invite), as one handler set per row. */
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcResult, MemberRow, VaultId } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { MemberHandlers } from "./MemberItem.tsx";

/** Runs a change and resolves to whether it went through (`useChange`). */
export type RunChange = <T>(change: () => Promise<IpcResult<T>>) => Promise<boolean>;

export function useMemberHandlers(
  vaultId: VaultId,
  run: RunChange,
  onVerify: (email: string) => void,
): (row: MemberRow) => MemberHandlers {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  return (row) => ({
    onRole: (role) => {
      void run(() => ipc.call("vault_member_set_role", { vaultId, accountId: row.accountId, role }));
    },
    onRemove: () => {
      void run(() => ipc.call("vault_member_remove", { vaultId, accountId: row.accountId })).then((done) => {
        if (done) {
          toast(t("sharing.members.removed"));
        }
      });
    },
    onCancelInvite: (invite) => {
      void run(() => ipc.call("vault_invite_cancel", { inviteId: invite.inviteId })).then((done) => {
        if (done) {
          toast(t("sharing.invite.cancelled"));
        }
      });
    },
    onVerify,
  });
}
