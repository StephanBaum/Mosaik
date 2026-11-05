/** One login in a list (E02): site tile, title and user, Fill, and a menu to copy the user name, password or code. */
import { useId } from "react";
import type { CopySource } from "../../shared/protocol.ts";
import { useI18n } from "../i18n.tsx";
import { Avatar, Button } from "./Button.tsx";
import { Menu, type MenuItem } from "./Menu.tsx";

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
  return [
    { id: "username", label: t("ext.item.copyUsername"), hint: "Ctrl+C", disabled: item.username === null, onSelect: copy("username") },
    { id: "password", label: t("ext.item.copyPassword"), hint: "Ctrl+Shift+C", onSelect: copy("password") },
    ...(item.hasOtp ? [{ id: "totp", label: t("ext.item.copyCode"), hint: "Ctrl+Alt+C", onSelect: copy("totp") }] : []),
  ];
}

export function ItemRow({
  item,
  primary,
  fillOff,
  onFill,
  onCopy,
}: {
  item: RowItem;
  /** The first match gets the holo button; the rest are plain. */
  primary: boolean;
  /** Fill is off for the whole page (a frame of another site has the focus). */
  fillOff: boolean;
  onFill: () => void;
  onCopy: (source: CopySource) => void;
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
        <span className="item__title">{item.title ?? name}</span>
        <span className="item__subtitle">{subtitleOf(item, t)}</span>
        {item.blocked && (
          <span id={noteId} className="item__note">
            {t("ext.item.blocked")}
          </span>
        )}
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
