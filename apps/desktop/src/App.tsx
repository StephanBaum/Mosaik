/**
 * The app shell: providers (IPC, settings, i18n, toasts, secrets, events) and the
 * session-driven routing between onboarding, unlock, the Emergency Kit recovery and the vault.
 */
import { useEffect } from "react";
import { EventsProvider } from "./app/EventsProvider.tsx";
import { SecretsProvider } from "./app/SecretsContext.tsx";
import { SettingsProvider, useSettings } from "./app/SettingsProvider.tsx";
import { ErrorAlert } from "./components/ErrorAlert.tsx";
import { ToastProvider } from "./components/ToastProvider.tsx";
import { I18nProvider, resolveLocale, useI18n } from "./i18n/I18nProvider.tsx";
import type { MosaicIpc } from "./ipc/client.ts";
import { IpcProvider, useIpc } from "./ipc/IpcProvider.tsx";
import { KitRecoveryFlow } from "./screens/KitRecoveryFlow.tsx";
import { OnboardingFlow } from "./screens/onboarding/OnboardingFlow.tsx";
import { UnlockScreen } from "./screens/UnlockScreen.tsx";
import { VaultLayout } from "./screens/vault/VaultLayout.tsx";
import { useSession, type SessionView } from "./session/useSession.ts";
import { applyTheme } from "./theme/theme.ts";

export function App({ ipc, languages = navigator.languages }: { ipc: MosaicIpc; languages?: readonly string[] }) {
  return (
    <IpcProvider ipc={ipc}>
      <SettingsProvider>
        <Localized languages={languages} />
      </SettingsProvider>
    </IpcProvider>
  );
}

function Localized({ languages }: { languages: readonly string[] }) {
  const { settings } = useSettings();
  const theme = settings?.theme ?? "system";
  useEffect(() => {
    applyTheme(theme);
  }, [theme]);
  return (
    <I18nProvider locale={resolveLocale(settings?.language ?? "system", languages)}>
      <ToastProvider>
        <SecretsProvider>
          <SessionRoot />
        </SecretsProvider>
      </ToastProvider>
    </I18nProvider>
  );
}

function SessionRoot() {
  const { t } = useI18n();
  const { view, setSession, reload } = useSession(useIpc());
  return (
    <EventsProvider onLocked={reload}>
      <a className="skip-link" href="#main">
        {t("app.skipToContent")}
      </a>
      <Routes view={view} setSession={setSession} />
    </EventsProvider>
  );
}

function Routes({
  view,
  setSession,
}: {
  view: SessionView;
  setSession: Parameters<typeof OnboardingFlow>[0]["onDone"];
}) {
  const { t } = useI18n();
  if (view.status === "loading") {
    return (
      <main id="main" className="screen screen--centered">
        <p className="status" aria-live="polite">
          {t("app.loading")}
        </p>
      </main>
    );
  }
  if (view.status === "failed") {
    return (
      <main id="main" className="screen screen--centered">
        <div className="panel">
          <h1 className="title">{t("app.name")}</h1>
          <ErrorAlert error={view.error} />
        </div>
      </main>
    );
  }
  const session = view.session;
  switch (session.state) {
    case "noAccount":
      return <OnboardingFlow onDone={setSession} />;
    case "locked":
      return <UnlockScreen session={session} onUnlocked={setSession} />;
    case "unlocked":
      return session.kitVerified ? (
        <VaultLayout onLocked={setSession} />
      ) : (
        <KitRecoveryFlow onDone={setSession} />
      );
  }
}
