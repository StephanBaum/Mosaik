/** The app sidebar (Main, D08): brand, categories, vaults, the other sections and the footer. */
import type { ReactNode } from "react";
import type { MessageKey } from "@mosaic/i18n";
import { useEvents } from "../../app/EventsProvider.tsx";
import { HoloSurface } from "../../components/HoloSurface.tsx";
import { Icon, type IconName } from "../../components/icons.tsx";
import { IconButton } from "../../components/IconButton.tsx";
import { Kbd } from "../../components/Kbd.tsx";
import { LogoMark } from "../../components/LogoMark.tsx";
import { useRail } from "../../hooks/useRail.ts";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { SyncState, SyncStatus as SyncStatusName, VaultId, VaultSummary } from "../../ipc/contract/index.ts";
import { usePlatform } from "../../platform/platform.ts";
import { ariaKeyshortcuts, format, SHORTCUTS } from "../../platform/shortcuts.ts";
import { CATEGORIES, type Category } from "./useVaultData.ts";
import type { View } from "./VaultLayout.tsx";
import { UserText } from "../../components/UserText.tsx";

const CATEGORY_ICONS: Record<Category, IconName> = {
  all: "layers",
  favorites: "star",
  logins: "key",
  passkeys: "passkey",
  codes: "timer",
  cards: "card",
  notes: "file",
  ssh: "terminal",
};

const VAULT_COLORS = 6;

const SYNC_STATUS: Record<SyncStatusName, { dot: string; label: MessageKey }> = {
  notConfigured: { dot: "", label: "sync.status.local" },
  upToDate: { dot: " sync-dot--ok", label: "sync.status.upToDate" },
  syncing: { dot: "", label: "sync.status.syncing" },
  offline: { dot: " sync-dot--offline", label: "sync.status.offline" },
  error: { dot: " sync-dot--error", label: "sync.status.error" },
};

function SyncStatus({ sync }: { sync: SyncState | null }) {
  const { t } = useI18n();
  const { dot, label } = SYNC_STATUS[sync?.status ?? "notConfigured"];
  return (
    <span className="sidebar__sync">
      <span className={`sync-dot${dot}`} aria-hidden="true" />
      <span className="sidebar__sync-text">{t(label)}</span>
    </span>
  );
}

export interface SidebarProps {
  vaults: readonly VaultSummary[];
  vaultId: VaultId | null;
  vaultName: string;
  view: View;
  category: Category;
  counts: Record<Category, number>;
  findingCount: number;
  onCategory: (category: Category) => void;
  onVault: (vaultId: VaultId) => void;
  onView: (view: View) => void;
  onLock: () => void;
  /** Opens the new-vault dialog. */
  onNewVault: () => void;
}

function Categories(props: SidebarProps) {
  const { t } = useI18n();
  const rail = useRail();
  return (
    <ul className="nav-group">
      {CATEGORIES.filter((category) => category === "all" || props.counts[category] > 0).map(
        (category) => (
          <li key={category}>
            <button
              type="button"
              className="nav-item"
              title={rail ? t(category === "all" ? "vault.allItems" : `vault.category.${category}`) : undefined}
              aria-current={props.view === "vault" && props.category === category ? "page" : undefined}
              onClick={() => {
                props.onCategory(category);
              }}
            >
              {props.view === "vault" && props.category === category && <HoloSurface className="nav-item__bar" />}
              <Icon name={CATEGORY_ICONS[category]} />
              <span className="nav-item__label">
                {t(category === "all" ? "vault.allItems" : `vault.category.${category}`)}
              </span>
              {props.counts[category] > 0 && (
                <span className="nav-item__count tabular">{props.counts[category]}</span>
              )}
            </button>
          </li>
        ),
      )}
    </ul>
  );
}

/** A closed hardware vault has no readable name: show the placeholder, not "Unnamed". */
export function vaultLabel(vault: VaultSummary, t: (key: MessageKey) => string): string {
  if (vault.name !== null) {
    return vault.name;
  }
  return t(vault.protection === "hardwareKey" ? "vault.hardware.placeholderName" : "vault.unnamed");
}

function Vaults(props: SidebarProps) {
  const { t } = useI18n();
  return (
    <div className="nav-group">
      <div className="nav-group__head">
        <h2 className="nav-group__title">{t("vault.vaults")}</h2>
        <IconButton icon="plus" size="sm" label={t("vault.create")} onClick={props.onNewVault} />
      </div>
      {props.vaults.map((vault, index) => (
        <button
          key={vault.vaultId}
          type="button"
          className="nav-item"
          // Names are cut off in the list: the full one is always on hover.
          title={vaultLabel(vault, t)}
          aria-current={props.vaultId === vault.vaultId && props.view === "vault" ? "page" : undefined}
          onClick={() => {
            props.onVault(vault.vaultId);
          }}
        >
          {props.vaultId === vault.vaultId && props.view === "vault" && (
            <HoloSurface className="nav-item__bar" />
          )}
          <span className="nav-item__slot">
            {vault.protection === "hardwareKey" ? (
              <Icon name="lock" size="sm" />
            ) : (
              <span className={`vault-dot vault-dot--${String(index % VAULT_COLORS)}`} aria-hidden="true" />
            )}
          </span>
          <span className="nav-item__label">
            <UserText>{vaultLabel(vault, t)}</UserText>
          </span>
          {vault.itemCount > 0 && <span className="nav-item__count tabular">{vault.itemCount}</span>}
        </button>
      ))}
    </div>
  );
}

function BottomGroup({ view, findingCount, onView }: SidebarProps) {
  const { t } = useI18n();
  const platform = usePlatform();
  const rail = useRail();
  const tip = (target: View, label: string) =>
    !rail ? undefined : target === "settings" ? `${label} (${format(SHORTCUTS.settings, platform, t)})` : label;
  const entry = (target: View, icon: IconName, label: string, extra?: ReactNode) => (
    <button
      type="button"
      className="nav-item"
      title={tip(target, label)}
      aria-current={view === target ? "page" : undefined}
      aria-keyshortcuts={target === "settings" ? ariaKeyshortcuts(SHORTCUTS.settings, platform) : undefined}
      onClick={() => {
        onView(target);
      }}
    >
      {view === target && <HoloSurface className="nav-item__bar" />}
      <Icon name={icon} />
      <span className="nav-item__label">{label}</span>
      {extra}
    </button>
  );
  return (
    <div className="nav-group nav-group--bottom">
      {entry(
        "security",
        "shield",
        t("nav.security"),
        findingCount > 0 && (
          <>
            <span className="chip nav-item__alert" aria-hidden="true">
              {findingCount}
            </span>
            <span className="visually-hidden">{t("nav.securityCount", { count: findingCount })}</span>
          </>
        ),
      )}
      {entry("trash", "trash", t("nav.trash"))}
      {entry("generator", "refresh", t("nav.generator"))}
      {entry("settings", "sliders", t("nav.settings"), <Kbd shortcut="settings" hidden />)}
    </div>
  );
}

export function Sidebar(props: SidebarProps) {
  const { t } = useI18n();
  const platform = usePlatform();
  const { sync } = useEvents();
  return (
    <nav className="sidebar" aria-label={t("vault.vaults")}>
      <div className="sidebar__brand">
        <LogoMark />
        <span className="sidebar__brand-text">
          <span className="sidebar__brand-name">{t("app.name")}</span>
          <span className="sidebar__brand-vault" title={props.vaultName}>
            <UserText>{props.vaultName}</UserText>
          </span>
        </span>
      </div>
      <Categories {...props} />
      <Vaults {...props} />
      <BottomGroup {...props} />
      <div className="sidebar__footer">
        <SyncStatus sync={sync} />
        <IconButton
          icon="lock"
          label={t("action.lock")}
          title={`${t("action.lock")} (${format(SHORTCUTS.lock, platform, t)})`}
          aria-keyshortcuts={ariaKeyshortcuts(SHORTCUTS.lock, platform)}
          data-reveal-origin="lock"
          onClick={props.onLock}
        />
      </div>
    </nav>
  );
}
