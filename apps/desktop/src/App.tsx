/**
 * The app shell: providers (IPC, i18n, theme) and the session-driven routing between
 * onboarding, unlock and the vault.
 */
import { useEffect, useState } from "react";
import { ErrorAlert } from "./components/ErrorAlert.tsx";
import { I18nProvider, resolveLocale, useI18n } from "./i18n/I18nProvider.tsx";
import type { MosaicIpc } from "./ipc/client.ts";
import type { Settings } from "./ipc/contract/index.ts";
import { IpcProvider, useIpc } from "./ipc/IpcProvider.tsx";
import { OnboardingScreen } from "./screens/OnboardingScreen.tsx";
import { UnlockScreen } from "./screens/UnlockScreen.tsx";
import { VaultScreen } from "./screens/VaultScreen.tsx";
import { useSession } from "./session/useSession.ts";
import { applyTheme } from "./theme/theme.ts";

export function App({ ipc, languages = navigator.languages }: { ipc: MosaicIpc; languages?: readonly string[] }) {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    void ipc.call("get_settings", null).then((result) => {
      if (result.ok) {
        setSettings(result.value);
      }
    });
  }, [ipc]);

  const theme = settings?.theme ?? "system";
  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  return (
    <IpcProvider ipc={ipc}>
      <I18nProvider locale={resolveLocale(settings?.language ?? "system", languages)}>
        <Shell />
      </I18nProvider>
    </IpcProvider>
  );
}

function Shell() {
  const { t } = useI18n();
  const { view, setSession } = useSession(useIpc());

  let screen;
  if (view.status === "loading") {
    screen = (
      <main id="main" className="screen screen--centered">
        <p className="status" aria-live="polite">
          {t("app.loading")}
        </p>
      </main>
    );
  } else if (view.status === "failed") {
    screen = (
      <main id="main" className="screen screen--centered">
        <div className="panel">
          <h1 className="title">{t("app.name")}</h1>
          <ErrorAlert error={view.error} />
        </div>
      </main>
    );
  } else {
    const session = view.session;
    switch (session.state) {
      case "noAccount":
        screen = <OnboardingScreen onDone={setSession} />;
        break;
      case "locked":
        screen = <UnlockScreen session={session} onUnlocked={setSession} />;
        break;
      case "unlocked":
        screen = <VaultScreen onLocked={setSession} />;
        break;
    }
  }

  return (
    <>
      <a className="skip-link" href="#main">
        {t("app.skipToContent")}
      </a>
      {screen}
    </>
  );
}
