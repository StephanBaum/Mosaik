/** One login in a list (E02): site tile, title and user, Fill, and a menu to copy the user name, password or code. */
import { useId } from "react";
import type { CopySource } from "../../shared/protocol.ts";
import { useI18n } from "../i18n.tsx";
import { Avatar, Button } from "./Button.tsx";
import { Menu, type MenuItem } from "./Menu.tsx";
import { shortcutPlatform } from "../platform.ts";

export interface RowItem {
  vaultId: string;
  itemId: string;
  title: string | null;
  username: string | null;
  hasOtp: boolean;
  /** Fill is offered (a search hit that does not fit this page is copy-only). */
  fillable: boolean;
  /** Why Fill is off although it would fit: the page is plain http and the login does not allow it. */
  blocked: boolean;
  /** Matched by the site's registrable domain, not the exact address. */
  sameSite: boolean;
  /** "Change password…" is offered: an exact address match on an https page. */
  canChange?: boolean;
}

export function rowName(item: RowItem, untitled: string): string {
  return item.username ?? item.title ?? untitled;
}

type Translate = ReturnType<typeof useI18n>["t"];

function subtitleOf(item: RowItem, t: Translate): string {
  return [
    item.username ?? t("ext.item.noUsername"),
    ...(item.hasOtp ? [t("ext.item.twoFactor")] : []),
    ...(item.sameSite ? [t("ext.logins.sameSite")] : []),
  ].join(" · ");
}

function copyItems(
  item: RowItem,
  t: Translate,
  copy: (field: "username" | "password" | "totp") => () => void,
): MenuItem[] {
  const platform = shortcutPlatform();
  return [
    {
      id: "username",
      label: t("ext.item.copyUsername"),
      hint: t("ext.shortcut.copyUser", { platform }),
      disabled: item.username === null,
      onSelect: copy("username"),
    },
    { id: "password", label: t("ext.item.copyPassword"), hint: t("ext.shortcut.copyPassword", { platform }), onSelect: copy("password") },
    ...(item.hasOtp
      ? [{ id: "totp", label: t("ext.item.copyCode"), hint: t("ext.shortcut.copyCode", { platform }), onSelect: copy("totp") }]
      : []),
  ];
}

/** "Change password…" under a login that matches this exact address. */
function ChangeLink({ name, onClick }: { name: string; onClick: () => void }) {
  const { t } = useI18n();
  return (
    <Button variant="link" className="item__action" aria-label={t("ext.rotation.changeLabel", { name })} onClick={onClick}>
      {t("ext.rotation.change")}
    </Button>
  );
}

/** What is said under a row: copy-only, blocked on http, and the change-password link. */
function Notes({
  item,
  name,
  noteId,
  onChangePassword,
}: {
  item: RowItem;
  name: string;
  noteId: string;
  onChangePassword: (() => void) | undefined;
}) {
  const { t } = useI18n();
  return (
    <>
      {!item.fillable && <span className="item__note item__note--quiet">{t("ext.search.copyOnly")}</span>}
      {item.blocked && (
        <span id={noteId} className="item__note">
          {t("ext.item.blocked")}
        </span>
      )}
      {item.canChange === true && onChangePassword !== undefined && (
        <ChangeLink name={name} onClick={onChangePassword} />
      )}
    </>
  );
}

export function ItemRow({
  item,
  primary,
  fillOff,
  onFill,
  onCopy,
  onChangePassword,
}: {
  item: RowItem;
  /** The first match gets the holo button, unless a save or code card above holds the screen's one primary; the rest are plain. */
  primary: boolean;
  /** Fill is off for the whole page (a frame of another site has the focus). */
  fillOff: boolean;
  onFill: () => void;
  onCopy: (source: CopySource) => void;
  /** Starts a password change for this login (the desktop dialog decides). */
  onChangePassword?: () => void;
}) {
  const { t } = useI18n();
  const noteId = useId();
  const name = rowName(item, t("ext.item.untitled"));
  const copy = (field: "username" | "password" | "totp") => () => {
    onCopy({ kind: "item", vaultId: item.vaultId, itemId: item.itemId, field });
  };
  const fillDisabled = item.blocked || fillOff;
  return (
    <li
      className={primary ? "item is-first" : "item"}
      data-vault-id={item.vaultId}
      data-item-id={item.itemId}
    >
      <Avatar text={item.title ?? name} active={primary} />
      <span className="item__text">
        <span className="item__title" title={item.title ?? name}>
          {item.title ?? name}
        </span>
        <span className="item__subtitle" title={subtitleOf(item, t)}>
          {subtitleOf(item, t)}
        </span>
        <Notes item={item} name={name} noteId={noteId} onChangePassword={onChangePassword} />
      </span>
      {item.fillable && (
        <Button
          variant={primary ? "primary" : "secondary"}
          aria-label={t("ext.item.fillLabel", { name })}
          aria-describedby={item.blocked ? noteId : undefined}
          disabled={fillDisabled}
          onClick={onFill}
        >
          {t("ext.item.fill")}
        </Button>
      )}
      <Menu icon="copy" label={t("ext.item.copyMenu", { name })} items={copyItems(item, t, copy)} />
    </li>
  );
}
