/** The item header: avatar, title, meta line and the actions (Main, D08, D19). */
import { useState } from "react";
import { Avatar } from "../../components/Avatar.tsx";
import { Button } from "../../components/Button.tsx";
import { Chip } from "../../components/Chip.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { Icon } from "../../components/icons.tsx";
import { IconButton } from "../../components/IconButton.tsx";
import { MenuButton, type MenuItem } from "../../components/Menu.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { ItemDetail as Detail, VaultSummary } from "../../ipc/contract/index.ts";
import { UserText } from "../../components/UserText.tsx";
import { titleOrUntitled } from "./title.ts";

export type ItemCommand = "trash_item" | "restore_item" | "purge_item";

export interface HeaderActions {
  onRun: (command: ItemCommand) => void;
  onFavorite: (favorite: boolean) => void;
  onMove: (vaultId: string) => void;
  onEdit: () => void;
}

/** Deleting for good cannot be undone: ask first, with Cancel focused. */
function PurgeDialog({
  title,
  onCancel,
  onConfirm,
}: {
  title: string;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  return (
    <Dialog title={t("trash.purgeConfirm.title", { title })} onClose={onCancel}>
      <p className="lede">{t("trash.purgeConfirm.body")}</p>
      <div className="dialog__actions">
        <Button data-autofocus="" onClick={onCancel}>
          {t("action.cancel")}
        </Button>
        <Button variant="dangerSolid" onClick={onConfirm}>
          {t("action.purge")}
        </Button>
      </div>
    </Dialog>
  );
}

function Actions({
  detail,
  trash,
  vaults,
  actions,
}: {
  detail: Detail;
  trash: boolean;
  vaults: readonly VaultSummary[];
  actions: HeaderActions;
}) {
  const { t } = useI18n();
  const [confirmPurge, setConfirmPurge] = useState(false);
  const summary = detail.summary;
  if (trash) {
    return (
      <div className="detail__actions">
        <Button
          onClick={() => {
            actions.onRun("restore_item");
          }}
        >
          <Icon name="restore" />
          {t("action.restore")}
        </Button>
        <Button
          variant="danger"
          onClick={() => {
            setConfirmPurge(true);
          }}
        >
          {t("action.purge")}
        </Button>
        {confirmPurge && (
          <PurgeDialog
            title={titleOrUntitled(summary.title, t("vault.untitled"))}
            onCancel={() => {
              setConfirmPurge(false);
            }}
            onConfirm={() => {
              setConfirmPurge(false);
              actions.onRun("purge_item");
            }}
          />
        )}
      </div>
    );
  }
  const items: MenuItem[] = [
    { label: t("action.delete"), tone: "danger", onSelect: () => { actions.onRun("trash_item"); } },
    ...vaults
      .filter((vault) => vault.vaultId !== summary.vaultId)
      .map((vault) => ({
        label: t("vault.moveToNamed", { name: vault.name ?? t("vault.unnamed") }),
        onSelect: () => {
          actions.onMove(vault.vaultId);
        },
      })),
  ];
  return (
    <div className="detail__actions">
      <IconButton
        icon="star"
        variant="outline"
        filled={summary.favorite}
        pressed={summary.favorite}
        label={t(summary.favorite ? "vault.unfavorite" : "vault.favorite")}
        onClick={() => {
          actions.onFavorite(!summary.favorite);
        }}
      />
      <Button onClick={actions.onEdit}>{t("action.edit")}</Button>
      <MenuButton label={t("action.moreActions")} items={items} />
    </div>
  );
}

export function DetailHeader({
  detail,
  trash,
  vaults,
  actions,
}: {
  detail: Detail;
  trash: boolean;
  vaults: readonly VaultSummary[];
  actions: HeaderActions;
}) {
  const { t } = useI18n();
  const summary = detail.summary;
  const title = titleOrUntitled(summary.title, t("vault.untitled"));
  const vaultName = vaults.find((vault) => vault.vaultId === summary.vaultId)?.name;
  return (
    <div className="detail__header">
      <Avatar size="lg" title={title} />
      <div className="detail__heading">
        <h2 className="display">
          <UserText>{title}</UserText>
        </h2>
        <div className="caption detail__meta">
          {trash && summary.trashedAt !== null ? (
            <span>
              {t("trash.detailMeta", {
                kind: t(`item.kind.${summary.kind}`),
                vault: vaultName ?? t("vault.unnamed"),
                deletedAt: new Date(summary.trashedAt),
              })}
            </span>
          ) : (
            <>
              <span>{t(`item.kind.${summary.kind}`)}</span>
              {vaultName != null && (
                <>
                  <span aria-hidden="true">·</span>
                  <span>
                    <UserText>{vaultName}</UserText>
                  </span>
                </>
              )}
            </>
          )}
          {summary.tags.map((tag) => (
            <Chip key={tag}>{tag}</Chip>
          ))}
        </div>
      </div>
      <Actions detail={detail} trash={trash} vaults={vaults} actions={actions} />
    </div>
  );
}
