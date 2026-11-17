/** The top bar (Main): breadcrumb, search, and the "New item" action. */
import { useEvents } from "../../app/EventsProvider.tsx";
import { Button } from "../../components/Button.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { useHoloLinger } from "../../holo/useHoloLinger.ts";
import { Icon } from "../../components/icons.tsx";
import { Kbd } from "../../components/Kbd.tsx";
import { MenuButton } from "../../components/Menu.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { usePlatform } from "../../platform/platform.ts";
import { ariaKeyshortcuts, SHORTCUTS } from "../../platform/shortcuts.ts";
import type { Category } from "./useVaultData.ts";
import type { View } from "./VaultLayout.tsx";

export interface TopBarProps {
  vaultName: string;
  view: View;
  category: Category;
  /** The editor's title while it is open (a third crumb). */
  editorTitle: string | null;
  query: string;
  headerPrimary: boolean;
  onQuery: (query: string) => void;
  onNew: () => void;
  onPalette: () => void;
  /** Opens the sharing dialog of the open vault (`undefined`: no single vault is open). */
  onShare?: (() => void) | undefined;
  /** Opens "Rotate vault key…" of the open vault (`undefined`: not offered here). */
  onRotate?: (() => void) | undefined;
}

function useCrumbs({ view, category, editorTitle, vaultName }: TopBarProps): string[] {
  const { t } = useI18n();
  const page = {
    vault: t(category === "all" ? "vault.allItems" : `vault.category.${category}`),
    trash: t("nav.trash"),
    security: t("nav.security"),
    generator: t("nav.generator"),
    settings: t("nav.settings"),
    import: t("import.flow.title"),
    invites: t("sharing.inbox.title"),
  }[view];
  return [vaultName, page, ...(editorTitle === null ? [] : [editorTitle])];
}

function Breadcrumb(props: TopBarProps) {
  const { t } = useI18n();
  const crumbs = useCrumbs(props);
  return (
    <nav aria-label={t("nav.breadcrumb")} className="breadcrumb">
      {crumbs.map((crumb, index) => (
        <span key={index} className="breadcrumb__item">
          {index > 0 && (
            <span aria-hidden="true" className="breadcrumb__sep">
              /
            </span>
          )}
          <span className="breadcrumb__crumb" aria-current={index === crumbs.length - 1 ? "page" : undefined}>
            {crumb}
          </span>
        </span>
      ))}
    </nav>
  );
}

/** An agent is using a key right now: a dot and a polite announcement, no sound. */
export function AgentActivityIndicator() {
  const { t } = useI18n();
  const { agent } = useEvents();
  return (
    <span className="agent-indicator" role="status" aria-live="polite">
      {agent.inFlight > 0 && (
        <>
          <span className="agent-indicator__dot" aria-hidden="true" />
          <span>{t("agent.inUse")}</span>
        </>
      )}
    </span>
  );
}

export function TopBar(props: TopBarProps) {
  const { t } = useI18n();
  const platform = usePlatform();
  const linger = useHoloLinger(props.headerPrimary);
  const NewButton = linger.holo ? HoloButton : Button;
  return (
    <header className="topbar">
      <Breadcrumb {...props} />
      <AgentActivityIndicator />
      <div className="search">
        <Icon name="search" />
        <input
          type="search"
          className="search__input"
          aria-label={t("vault.search")}
          placeholder={t("vault.search")}
          value={props.query}
          onChange={(event) => {
            props.onQuery(event.target.value);
          }}
        />
        <button
          type="button"
          className="kbd-button"
          title={t("palette.open")}
          aria-keyshortcuts={ariaKeyshortcuts(SHORTCUTS.palette, platform)}
          onClick={props.onPalette}
        >
          <span className="visually-hidden">{t("palette.open")}</span>
          <Kbd shortcut="palette" />
        </button>
      </div>
      {props.onShare !== undefined && (
        <Button className="topbar__share" onClick={props.onShare}>
          <Icon name="users" />
          <span className="topbar__new-label">{t("sharing.share.open")}</span>
        </Button>
      )}
      {props.onRotate !== undefined && (
        <MenuButton
          label={t("vault.menu")}
          items={[{ label: t("vaultKey.menu"), onSelect: props.onRotate }]}
        />
      )}
      <NewButton
        data-new-item=""
        className="topbar__new"
        onClick={() => {
          linger.pressed();
          props.onNew();
        }}
        aria-keyshortcuts={ariaKeyshortcuts(SHORTCUTS.newItem, platform)}
      >
        <Icon name="plus" />
        <span className="topbar__new-label">{t("vault.new")}</span>
      </NewButton>
    </header>
  );
}
