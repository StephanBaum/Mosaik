/** One item: text fields, secrets (revealed on request), the one-time code and actions. */
import { useEffect, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { CopyButton, useCopy } from "../../components/CopyButton.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { SecretField } from "../../components/SecretField.tsx";
import { Select } from "../../components/Select.tsx";
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
import { TotpCode } from "./TotpCode.tsx";

type ItemCommand = "trash_item" | "restore_item" | "purge_item";

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

function Fields({ detail, item }: { detail: Detail; item: ItemRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  return (
    <>
      <dl className="fields">
        {detail.text.map((entry) => {
          const label = t(`item.field.${entry.field}`);
          return (
            <div key={entry.field} className="fields__row">
              <dt>{label}</dt>
              <dd>{entry.value}</dd>
              {entry.field === "username" && (
                <dd className="fields__actions">
                  <CopyButton source={{ kind: "item", ...item, field: { kind: "username" } }} field={label} />
                </dd>
              )}
            </div>
          );
        })}
        {detail.urls.length > 0 && (
          <div className="fields__row">
            <dt>{t("item.urls", { count: detail.urls.length })}</dt>
            <dd>{detail.urls.map((u) => u.url).join(", ")}</dd>
          </div>
        )}
      </dl>
      {detail.secrets.map((field) => {
        const label = t(`item.field.${field}`);
        const copyField = field === "password" ? ({ kind: "password" } as const) : ({ kind: "secret", field } as const);
        return (
          <SecretField
            key={field}
            label={label}
            reveal={() => ipc.call("reveal_secret", { ...item, secret: { kind: "field", field } })}
            actions={<CopyButton source={{ kind: "item", ...item, field: copyField }} field={label} />}
          />
        );
      })}
      {detail.otp !== null && <TotpCode item={item} />}
    </>
  );
}

function Actions({
  detail,
  trash,
  vaults,
  onRun,
  onFavorite,
  onMove,
  onEdit,
}: {
  detail: Detail;
  trash: boolean;
  vaults: readonly VaultSummary[];
  onRun: (command: ItemCommand) => void;
  onFavorite: (favorite: boolean) => void;
  onMove: (vaultId: string) => void;
  onEdit: () => void;
}) {
  const { t } = useI18n();
  const summary = detail.summary;
  if (trash) {
    return (
      <div className="actions">
        <Button onClick={() => { onRun("restore_item"); }}>{t("action.restore")}</Button>
        <Button onClick={() => { onRun("purge_item"); }}>{t("action.purge")}</Button>
      </div>
    );
  }
  const others = vaults.filter((vault) => vault.vaultId !== summary.vaultId);
  return (
    <div className="actions">
      <Button onClick={onEdit}>{t("action.edit")}</Button>
      <Button aria-pressed={summary.favorite} onClick={() => { onFavorite(!summary.favorite); }}>
        {t(summary.favorite ? "vault.unfavorite" : "vault.favorite")}
      </Button>
      <Button onClick={() => { onRun("trash_item"); }}>{t("action.delete")}</Button>
      {others.length > 0 && (
        <Select
          label={t("vault.moveTo")}
          value=""
          options={[
            { value: "", label: "–" },
            ...others.map((vault) => ({ value: vault.vaultId, label: vault.name ?? t("vault.unnamed") })),
          ]}
          onChange={(event) => {
            if (event.target.value !== "") {
              onMove(event.target.value);
            }
          }}
        />
      )}
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
  return (
    <article className="detail" aria-label={title}>
      <h2 className="detail__title">{title}</h2>
      <p className="muted">{t(`item.kind.${detail.summary.kind}`)}</p>
      {detail.review.conflictingFields > 0 && <p className="alert">{t("item.review")}</p>}
      <ErrorAlert error={error} />
      <Fields detail={detail} item={item} />
      <Actions
        detail={detail}
        trash={trash}
        vaults={vaults}
        onRun={(command) => {
          void ipc.call(command, item).then((result) => settle(result, true, messages[command]));
        }}
        onFavorite={(favorite) => {
          void ipc.call("set_favorite", { ...item, favorite }).then((result) => settle(result, false));
        }}
        onMove={(targetVaultId) => {
          void ipc.call("move_item", { ...item, targetVaultId }).then((result) => settle(result, true));
        }}
        onEdit={() => {
          onEdit(detail);
        }}
      />
    </article>
  );
}
