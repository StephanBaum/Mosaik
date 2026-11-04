/**
 * The item editor's form: its state shape, the draft sent to Rust and the save flow,
 * including a one-time code kept in a hardware-protected vault (ADR-0031).
 */
import { useState } from "react";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type {
  IpcError,
  ItemDetail,
  ItemDraft,
  ItemKind,
  ItemRequest,
  VaultSummary,
} from "../../ipc/contract/index.ts";
import { OTP_UNCHANGED, type OtpFormValue } from "./otp/OtpSection.tsx";

export interface Form {
  kind: ItemKind;
  vault: string;
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
  tags: string;
  /** What to do with the one-time code (ADR-0031). */
  otp: OtpFormValue;
}

function textOf(detail: ItemDetail | null, field: string): string {
  return detail?.text.find((entry) => entry.field === field)?.value ?? "";
}

export function initialForm(editing: ItemDetail | null, vaults: readonly VaultSummary[], vaultId: string | null): Form {
  return {
    kind: editing?.summary.kind ?? "login",
    vault: vaultId ?? vaults[0]?.vaultId ?? "",
    title: editing?.summary.title ?? "",
    username: textOf(editing, "username"),
    password: "",
    url: "",
    notes: textOf(editing, "notes"),
    tags: editing?.summary.tags.join(", ") ?? "",
    otp: OTP_UNCHANGED,
  };
}

/** What the editor sends for `otp`; a code kept in a hardware vault is saved separately. */
function otpChange(value: OtpFormValue): ItemDraft["otp"] {
  if (value.kind === "remove") {
    return { op: "remove" };
  }
  if (value.kind === "pending" && value.result.target.kind === "item") {
    const { pending, issuer, accountName } = value.result;
    return { op: "setPending", pendingId: pending.pendingId, issuer, accountName };
  }
  return null;
}

function buildDraft(form: Form): ItemDraft {
  const empty = (value: string): string | null => (value === "" ? null : value);
  return {
    text: [
      { field: "title", value: form.title },
      { field: "username", value: empty(form.username) },
      { field: "notes", value: empty(form.notes) },
    ],
    secrets: form.password === "" ? [] : [{ field: "password", value: form.password }],
    urls: form.url === "" ? [] : [{ op: "add", url: form.url, matchMode: "exactOrigin", allowHttp: false }],
    customFields: [],
    tags: form.tags
      .split(",")
      .map((tag) => tag.trim())
      .filter((tag) => tag !== ""),
    otp: otpChange(form.otp),
  };
}

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
    },
  });
  return stored.ok ? null : stored.error;
}

export function useSave(
  editing: ItemDetail | null,
  vaults: readonly VaultSummary[],
  onSaved: (item: ItemRequest) => void,
  onCodeStored: () => void,
) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [error, setError] = useState<IpcError | null>(null);

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
      onCodeStored();
      const vault = vaults.find((each) => each.vaultId === vaultId);
      toast(
        t("otp.toast.savedInVault", { vault: vault?.name ?? t("vault.hardware.placeholderName") }),
      );
    }
    const draft = buildDraft(form);
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
      onSaved({ vaultId: result.value.summary.vaultId, itemId: result.value.summary.itemId });
    } else {
      setError(result.error);
    }
  }
  return { error, setError, save };
}
