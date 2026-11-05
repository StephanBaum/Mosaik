/** The popup: chooses the view from the link to the desktop, and gives every screen the same services. */
import { negotiateLocale } from "@mosaic/i18n";
import { useMemo, useState } from "react";
import type { PopupState } from "../shared/messages.ts";
import type { PopupApi } from "./client.ts";
import { AnnouncerProvider } from "./components/Announcer.tsx";
import { Footer, Header, Logo, Shell, type View } from "./components/Shell.tsx";
import type { PopupEnv } from "./env.ts";
import { I18nProvider, useI18n } from "./i18n.tsx";
import { ConnectView } from "./screens/ConnectView.tsx";
import { GeneratorView } from "./screens/GeneratorView.tsx";
import { LockedView } from "./screens/LockedView.tsx";
import { LoginsView } from "./screens/LoginsView.tsx";
import { ServicesProvider, usePopupState } from "./usePopup.ts";

function Body({ state, view }: { state: PopupState; view: View }) {
  const { t } = useI18n();
  switch (state.connection) {
    case "unlocked":
      return view === "generator" ? <GeneratorView state={state} /> : <LoginsView state={state} />;
    case "locked":
      return <LockedView state={state} />;
    case "checking":
      return <p className="muted center">{t("ext.connect.checking")}</p>;
    default:
      return <ConnectView state={state} />;
  }
}

function Popup({ api, env }: { api: PopupApi; env: PopupEnv }) {
  const { t, errorText } = useI18n();
  const { state, loadError, refresh } = usePopupState(api);
  const [view, setView] = useState<View>("logins");
  const services = useMemo(() => ({ api, env, refresh }), [api, env, refresh]);

  if (state === null) {
    return (
      <div className="popup">
        <header className="header">
          <Logo />
        </header>
        <main id="main" className="main">
          {loadError === null ? (
            <p className="muted" role="status">
              {t("ext.loading")}
            </p>
          ) : (
            <p className="alert" role="alert">
              {errorText(loadError)}
            </p>
          )}
        </main>
      </div>
    );
  }
  return (
    <ServicesProvider value={services}>
      <Shell header={<Header state={state} view={view} onView={setView} />} footer={<Footer state={state} />}>
        <Body state={state} view={view} />
      </Shell>
    </ServicesProvider>
  );
}

export function App({ api, env, languages }: { api: PopupApi; env: PopupEnv; languages: readonly string[] }) {
  return (
    <I18nProvider locale={negotiateLocale(languages)}>
      <AnnouncerProvider>
        <Popup api={api} env={env} />
      </AnnouncerProvider>
    </I18nProvider>
  );
}
