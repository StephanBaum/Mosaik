/** One item (Main, D08): header, field rows, notes and a footer line. */
import { useEffect, useState } from "react";
import { Callout } from "../../components/Callout.tsx";
import { useCopy } from "../../components/CopyButton.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n, type I18n } from "../../i18n/I18nProvider.tsx";
import { useReauth } from "../../hooks/useReauth.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type {
  CopyField,
  IpcError,
  ItemDetail as Detail,
  ItemRequest,
  VaultSummary,
} from "../../ipc/contract/index.ts";
import { usePlatform } from "../../platform/platform.ts";
import { useAppShortcuts } from "../../platform/shortcuts.ts";
import { DetailFields } from "./DetailFields.tsx";
import { DetailHeader, type ItemCommand } from "./DetailHeader.tsx";
import { DetailRotation } from "./DetailRotation.tsx";
import { FilesSection } from "./files/FilesSection.tsx";
import { titleOrUntitled } from "./title.ts";

function useDetail(item: ItemRequest, reloadKey: number) {
  const ipc = useIpc();
  const [detail, setDetail] = useState<Detail | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  useEffect(() => {
    let current = true;
    void ipc.call("get_item", item).then((result) => {
      if (current) {
        setDetail(result.ok ? result.value : null);
        setError(result.ok ? null : result.error);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, item, reloadKey]);
  return { detail, setDetail, error, setError };
}

/** Whether a copy shortcut would fight the user's own copy: selected text, a field or a dialog. */
function copyShortcutBlocked(): boolean {
  const focus = document.activeElement;
  const typing =
    focus instanceof HTMLInputElement ||
    focus instanceof HTMLTextAreaElement ||
    (focus instanceof HTMLElement && focus.isContentEditable);
  return (
    (window.getSelection()?.toString() ?? "") !== "" ||
    typing ||
    document.querySelector("dialog[open]") !== null
  );
}

/** Ctrl+C, Ctrl+Shift+C and Ctrl+Alt+C copy user name, password and one-time code (UX §4). */
function useCopyShortcuts(item: ItemRequest, detail: Detail | null, enabled: boolean): void {
  const copy = useCopy();
  const platform = usePlatform();
  const guarded = (field: CopyField, available: boolean) => () => {
    if (available && !copyShortcutBlocked()) {
      void copy({ kind: "item", ...item, field });
    }
  };
  useAppShortcuts(
    platform,
    enabled && detail !== null
      ? {
          copyUsername: guarded(
            { kind: "username" },
            detail.text.some((entry) => entry.field === "username"),
          ),
          copyPassword: guarded({ kind: "password" }, detail.secrets.includes("password")),
          copyTotp: guarded({ kind: "totp" }, detail.otp !== null),
        }
      : {},
  );
}

/** The toast of each item command. */
function commandMessages(t: I18n["t"]): Record<ItemCommand, string> {
  return {
    trash_item: t("toast.trashed"),
    restore_item: t("toast.restored"),
    purge_item: t("toast.purged"),
  };
}

function DetailNotes({ notes }: { notes: string | undefined }) {
  const { t } = useI18n();
  if (notes === undefined || notes === "") {
    return null;
  }
  return (
    <section className="detail__notes">
      <h3 className="row__label">{t("item.field.notes")}</h3>
      <p className="detail__notes-text">{notes}</p>
    </section>
  );
}

function DetailFooter({ detail }: { detail: Detail }) {
  const { t } = useI18n();
  const { modifiedAt } = detail.summary;
  return (
    <div className="detail__footer">
      {modifiedAt !== null && <span>{t("item.changedAt", { at: new Date(modifiedAt) })}</span>}
      {detail.createdAt !== null && <span>{t("item.createdAt", { at: new Date(detail.createdAt) })}</span>}
      {detail.historyCount > 0 && <span>{t("item.historyCount", { count: detail.historyCount })}</span>}
    </div>
  );
}

/**
 * The fields, the files, the notes and the footer. A document is its files, so they come first
 * (and take the cursor when the document is new); on other items they follow the fields.
 */
function DetailBody({
  detail,
  item,
  trash,
  justCreated,
  onDetail,
}: {
  detail: Detail;
  item: ItemRequest;
  trash: boolean;
  justCreated: boolean;
  onDetail: (detail: Detail) => void;
}) {
  const isDocument = detail.summary.kind === "document";
  const notes = detail.text.find((entry) => entry.field === "notes")?.value;
  const files = (
    <FilesSection
      detail={detail}
      item={item}
      // A passkey has no files: its fields come from the site that made it.
      editable={!trash && detail.summary.kind !== "passkey"}
      autoFocusAdd={isDocument && justCreated}
      quietWhenEmpty={!isDocument}
      onChanged={onDetail}
    />
  );
  return (
    <>
      {isDocument && files}
      <DetailFields detail={detail} item={item} />
      <DetailRotation detail={detail} item={item} trash={trash} onDetail={onDetail} />
      {!isDocument && files}
      <DetailNotes notes={notes} />
      <DetailFooter detail={detail} />
    </>
  );
}

export function ItemDetail({
  item,
  trash,
  vaults,
  reloadKey,
  justCreated = false,
  onChanged,
  onEdit,
}: {
  item: ItemRequest;
  trash: boolean;
  vaults: readonly VaultSummary[];
  reloadKey: number;
  /** The item was saved a moment ago: a document puts the cursor on "Add file". */
  justCreated?: boolean;
  /** Something changed; `gone` when the item no longer exists under this ID. */
  onChanged: (gone: boolean) => void;
  onEdit: (detail: Detail) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const { detail, setDetail, error, setError } = useDetail(item, reloadKey);
  // Moving an item out of a hardware vault needs a fresh master password.
  const { guard, dialog: reauthDialog } = useReauth("reauth.bodyMove", "leaveHardwareVault");
  useCopyShortcuts(item, detail, !trash);

  const messages = commandMessages(t);

  async function settle(result: { ok: true } | { ok: false; error: IpcError }, gone: boolean, message?: string) {
    if (result.ok) {
      if (message !== undefined) {
        toast(message);
      }
      onChanged(gone);
    } else if (result.error.code !== "cancelled") {
      // Giving up the master-password step on purpose is not an error.
      setError(result.error);
    }
    await Promise.resolve();
  }

  if (detail === null) {
    return <ErrorAlert error={error} />;
  }
  const title = titleOrUntitled(detail.summary.title, t("vault.untitled"));
  return (
    <article className="detail" aria-label={title}>
      <DetailHeader
        detail={detail}
        trash={trash}
        vaults={vaults}
        actions={{
          onRun: (command) => {
            void ipc.call(command, item).then((result) => settle(result, true, messages[command]));
          },
          onFavorite: (favorite) => {
            void ipc.call("set_favorite", { ...item, favorite }).then((result) => settle(result, false));
          },
          onMove: (targetVaultId) => {
            void guard(() => ipc.call("move_item", { ...item, targetVaultId })).then((result) =>
              settle(result, true),
            );
          },
          onEdit: () => {
            onEdit(detail);
          },
        }}
      />
      {detail.review.conflictingFields > 0 && <Callout tone="warning">{t("item.review")}</Callout>}
      <ErrorAlert error={error} />
      <DetailBody
        detail={detail}
        item={item}
        trash={trash}
        justCreated={justCreated}
        onDetail={(changed) => {
          setDetail(changed);
          onChanged(false);
        }}
      />
      {reauthDialog}
    </article>
  );
}
