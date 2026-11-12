/** The popup frame (mockups E01 to E07): header with the site and the two views, the page, the footer. */
import { useState, type ReactNode } from "react";
import { errorCodeOf } from "../../shared/errors.ts";
import type { PopupRequest, PopupState } from "../../shared/messages.ts";
import { useI18n } from "../i18n.tsx";
import { useActions } from "../useActions.ts";
import { useServices } from "../usePopup.ts";
import { StatusBar, useAnnounce } from "./Announcer.tsx";
import { IconButton } from "./Button.tsx";
import { ConfirmDialog } from "./Dialog.tsx";
import { Icon } from "./Icon.tsx";
import { Menu, type MenuItem } from "./Menu.tsx";
import { NeverSaveDialog } from "./NeverSaveDialog.tsx";

export type View = "logins" | "generator";

/** The Mosaic mark: four tiles, the last one in the accent. */
export function Logo({ size = 18 }: { size?: number }) {
  const { t } = useI18n();
  return (
    <svg className="logo" width={size} height={size} viewBox="0 0 20 20" role="img" aria-label={t("ext.manifest.name")}>
      <rect x="0" y="0" width="9" height="9" rx="2.5" />
      <rect x="11" y="0" width="9" height="9" rx="2.5" />
      <rect x="0" y="11" width="9" height="9" rx="2.5" />
      <rect x="11" y="11" width="9" height="9" rx="2.5" className="logo__accent" />
    </svg>
  );
}

function SiteChip({ tab }: { tab: PopupState["tab"] }) {
  const { t } = useI18n();
  if (tab === null) {
    return (
      <span className="chip chip--quiet">
        <span className="visually-hidden">{t("ext.site.none")}</span>
      </span>
    );
  }
  return (
    <span className="chip">
      <Icon name={tab.secure ? "lock" : "alert"} size="sm" />
      <span className="visually-hidden">
        {t(tab.secure ? "ext.site.secure" : "ext.site.insecure", { host: "" })}
      </span>
      <span className="chip__host">{tab.host}</span>
    </span>
  );
}

export function Header({
  state,
  view,
  onView,
}: {
  state: PopupState;
  view: View;
  onView: (view: View) => void;
}) {
  const { t } = useI18n();
  return (
    <header className="header">
      <Logo />
      <SiteChip tab={state.tab} />
      <span className="grow" />
      {state.connection === "unlocked" && (
        <nav className="header__nav" aria-label={t("ext.nav.label")}>
          <IconButton
            icon="search"
            label={t("ext.nav.logins")}
            current={view === "logins"}
            onClick={() => {
              onView("logins");
            }}
          />
          <IconButton
            icon="key"
            label={t("ext.nav.generator")}
            current={view === "generator"}
            onClick={() => {
              onView("generator");
            }}
          />
        </nav>
      )}
    </header>
  );
}

/**
 * A More-menu switch that needs the optional site access: the access is asked for here, in the
 * click, then the background is told. The access is shared with the other switch, so it is
 * given back only when neither needs it.
 */
function useSiteAccessItem(config: {
  id: string;
  label: string;
  description: string;
  denied: string | null;
  on: boolean;
  otherOn: boolean;
  request: (enabled: boolean) => PopupRequest;
}): MenuItem {
  const { errorText } = useI18n();
  const { api, env, refresh } = useServices();
  const announce = useAnnounce();
  return {
    id: config.id,
    label: config.label,
    description: config.description,
    checked: config.on,
    onSelect: () => {
      const enable = !config.on;
      void (async () => {
        try {
          if (enable && !(await env.requestSiteAccess())) {
            announce(config.denied ?? errorText("permissionDenied"));
            return;
          }
          await api.request(config.request(enable));
          if (!enable && !config.otherOn) {
            await env.removeSiteAccess();
          }
          await refresh();
        } catch (error) {
          announce(errorText(errorCodeOf(error)));
        }
      })();
    },
  };
}

function DisconnectDialog({ onDone }: { onDone: () => void }) {
  const { t } = useI18n();
  const { api, refresh } = useServices();
  return (
    <ConfirmDialog
      title={t("ext.more.disconnectTitle")}
      confirmLabel={t("ext.more.disconnect")}
      onCancel={onDone}
      onConfirm={() => {
        onDone();
        void api.request({ op: "unpair" }).finally(() => refresh());
      }}
    >
      <p>{t("ext.more.disconnectBody")}</p>
    </ConfirmDialog>
  );
}

function footerStatus(connection: PopupState["connection"]) {
  if (connection === "unlocked") {
    return { key: "ext.footer.connected" as const, tone: "on" };
  }
  return connection === "locked"
    ? { key: "ext.footer.locked" as const, tone: "off" }
    : { key: "ext.footer.offline" as const, tone: "off" };
}

export function Footer({ state }: { state: PopupState }) {
  const { t } = useI18n();
  const { openDesktop } = useActions();
  const [confirming, setConfirming] = useState(false);
  const [listing, setListing] = useState(false);
  const saveOffers = useSiteAccessItem({
    id: "saveOffers",
    label: t("ext.more.saveOffers"),
    description: t("ext.more.saveOffersHint"),
    denied: null,
    on: state.saveOffers,
    otherOn: state.passkeys,
    request: (enabled) => ({ op: "saveOffersChanged", enabled }),
  });
  const passkeys = useSiteAccessItem({
    id: "passkeys",
    label: t("ext.passkeys.toggle"),
    description: t("ext.passkeys.hint"),
    denied: t("ext.passkeys.permissionDenied"),
    on: state.passkeys,
    otherOn: state.saveOffers,
    request: (enabled) => ({ op: "setPasskeys", enabled }),
  });
  const paired = state.connection === "locked" || state.connection === "unlocked";
  const items: MenuItem[] = [
    ...(paired || state.connection === "desktopNotRunning" || state.connection === "error"
      ? [
          saveOffers,
          {
            id: "neverSave",
            label: t("ext.neverSave.menu"),
            onSelect: () => {
              setListing(true);
            },
          },
          passkeys,
        ]
      : []),
    ...(paired
      ? [
          {
            id: "disconnect",
            label: t("ext.more.disconnect"),
            onSelect: () => {
              setConfirming(true);
            },
          },
        ]
      : []),
  ];
  const status = footerStatus(state.connection);
  return (
    <footer className="footer">
      <button type="button" className="footer__open" onClick={() => void openDesktop()}>
        {t("ext.footer.open")}
        <Icon name="external" size="sm" />
      </button>
      <span className="grow" />
      <span className={`dot dot--${status.tone}`} aria-hidden="true" />
      <span className="footer__status">{t(status.key)}</span>
      {items.length > 0 && (
        <Menu icon="more" label={t("ext.footer.more")} placement="above" items={items} />
      )}
      {listing && (
        <NeverSaveDialog
          onClose={() => {
            setListing(false);
          }}
        />
      )}
      {confirming && (
        <DisconnectDialog
          onDone={() => {
            setConfirming(false);
          }}
        />
      )}
    </footer>
  );
}

export function Shell({ header, footer, children }: { header: ReactNode; footer: ReactNode; children: ReactNode }) {
  return (
    <div className="popup">
      {header}
      <main id="main" className="main">
        {children}
      </main>
      <StatusBar />
      {footer}
    </div>
  );
}
