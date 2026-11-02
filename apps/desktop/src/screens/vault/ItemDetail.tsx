/** One item (Main, D08): header, field rows, notes and a footer line. */
import { useEffect, useState } from "react";
import { Callout } from "../../components/Callout.tsx";
import { useCopy } from "../../components/CopyButton.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
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
  return { detail, error, setError };
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

export function ItemDetail({
  item,
  trash,
  vaults,
  reloadKey,
  onChanged,
  onEdit,
}: {
  item: ItemRequest;
  trash: boolean;
  vaults: readonly VaultSummary[];
  reloadKey: number;
  /** Something changed; `gone` when the item no longer exists under this ID. */
  onChanged: (gone: boolean) => void;
  onEdit: (detail: Detail) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const { detail, error, setError } = useDetail(item, reloadKey);
  useCopyShortcuts(item, detail, !trash);

  const messages: Record<ItemCommand, string> = {
    trash_item: t("toast.trashed"),
    restore_item: t("toast.restored"),
    purge_item: t("toast.purged"),
  };

  async function settle(result: { ok: true } | { ok: false; error: IpcError }, gone: boolean, message?: string) {
    if (result.ok) {
      if (message !== undefined) {
        toast(message);
      }
      onChanged(gone);
    } else {
      setError(result.error);
    }
    await Promise.resolve();
  }

  if (detail === null) {
    return <ErrorAlert error={error} />;
  }
  const title = detail.summary.title ?? t("vault.untitled");
  const notes = detail.text.find((entry) => entry.field === "notes")?.value;
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
            void ipc.call("move_item", { ...item, targetVaultId }).then((result) => settle(result, true));
          },
          onEdit: () => {
            onEdit(detail);
          },
        }}
      />
      {detail.review.conflictingFields > 0 && <Callout tone="warning">{t("item.review")}</Callout>}
      <ErrorAlert error={error} />
      <DetailFields detail={detail} item={item} />
      {notes != null && notes !== "" && (
        <section className="detail__notes">
          <h3 className="row__label">{t("item.field.notes")}</h3>
          <p className="detail__notes-text">{notes}</p>
        </section>
      )}
      <DetailFooter detail={detail} />
    </article>
  );
}
