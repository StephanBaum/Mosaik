/**
 * The editor's save flow, including a one-time code kept in a hardware-protected vault
 * (ADR-0031).
 */
import { useRef, useState } from "react";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { IpcError, ItemDetail, ItemRequest, ItemSummary, VaultSummary } from "../../ipc/contract/index.ts";
import { buildDraft, type Form } from "./editorForm.ts";
import { OTP_UNCHANGED } from "./otp/OtpSection.tsx";

/**
 * A code the user wants in a hardware-protected vault becomes its own one-time code item
 * there, before the login is saved. A failed call keeps the pending setup in Rust.
 */
async function storeCodeInVault(
  ipc: ReturnType<typeof useIpc>,
  form: Form,
): Promise<IpcError | null> {
  if (form.otp.kind !== "pending" || form.otp.result.target.kind !== "hardwareVault") {
    return null;
  }
  const { pending, issuer, accountName, target } = form.otp.result;
  const stored = await ipc.call("add_item", {
    vaultId: target.vaultId,
    kind: "totp",
    draft: {
      text: [{ field: "title", value: form.title }],
      secrets: [],
      urls: [],
      customFields: [],
      tags: null,
      otp: { op: "setPending", pendingId: pending.pendingId, issuer, accountName },
      sshKey: null,
    },
  });
  return stored.ok ? null : stored.error;
}

export function useSave({
  editing,
  vaults,
  initial,
  onSaved,
  onCodeStored,
}: {
  editing: ItemDetail | null;
  vaults: readonly VaultSummary[];
  /** The form the editor started with: the draft holds what differs from it. */
  initial: Form;
  /**
   * The saved item and its list row (the list updates in place from it). `codeStored`: a code
   * went into a hardware vault as an item of its own, which the in-place update does not show.
   */
  onSaved: (item: ItemRequest, summary: ItemSummary, codeStored: boolean) => void;
  onCodeStored: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [error, setError] = useState<IpcError | null>(null);
  // Set once a code went into a hardware vault, also across a retry of the login.
  const codeInVault = useRef(false);

  async function save(submitted: Form) {
    setError(null);
    const failure = await storeCodeInVault(ipc, submitted);
    if (failure !== null) {
      setError(failure);
      return;
    }
    let form = submitted;
    if (submitted.otp.kind === "pending" && submitted.otp.result.target.kind === "hardwareVault") {
      // Stored: a retry of the login must not add the code twice.
      const { vaultId } = submitted.otp.result.target;
      form = { ...submitted, otp: OTP_UNCHANGED };
      codeInVault.current = true;
      onCodeStored();
      const vault = vaults.find((each) => each.vaultId === vaultId);
      toast(
        t("otp.toast.savedInVault", { vault: vault?.name ?? t("vault.hardware.placeholderName") }),
      );
    }
    const draft = buildDraft(form, initial);
    const result =
      editing === null
        ? await ipc.call("add_item", { vaultId: form.vault, kind: form.kind, draft })
        : await ipc.call("update_item", {
            vaultId: editing.summary.vaultId,
            itemId: editing.summary.itemId,
            draft,
          });
    if (result.ok) {
      toast(t("toast.saved"));
      const { summary } = result.value;
      onSaved({ vaultId: summary.vaultId, itemId: summary.itemId }, summary, codeInVault.current);
    } else {
      setError(result.error);
    }
  }
  return { error, setError, save };
}
