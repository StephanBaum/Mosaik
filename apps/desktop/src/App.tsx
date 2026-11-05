/**
 * The app shell: providers (IPC, settings, i18n, toasts, secrets, events) and the
 * session-driven routing between onboarding, unlock, the Emergency Kit recovery and the vault.
 */
import { useEffect, useState } from "react";
import { EventsProvider } from "./app/EventsProvider.tsx";
import { SecretsProvider } from "./app/SecretsContext.tsx";
import { SettingsProvider, useSettings } from "./app/SettingsProvider.tsx";
import { ErrorAlert } from "./components/ErrorAlert.tsx";
import { HoloProvider } from "./holo/HoloProvider.tsx";
import { ToastProvider } from "./components/ToastProvider.tsx";
import { useEffectiveTheme } from "./hooks/useEffectiveTheme.ts";
import { I18nProvider, resolveLocale, useI18n } from "./i18n/I18nProvider.tsx";
import type { MosaicIpc } from "./ipc/client.ts";
import { IpcProvider, useIpc } from "./ipc/IpcProvider.tsx";
import { KitRecoveryFlow } from "./screens/KitRecoveryFlow.tsx";
import { OnboardingFlow } from "./screens/onboarding/OnboardingFlow.tsx";
import { RevealProvider } from "./reveal/RevealProvider.tsx";
import { RevealStage, type RevealTarget } from "./reveal/RevealStage.tsx";
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
  const dark = useEffectiveTheme(theme) === "dark";
  useEffect(() => {
    applyTheme(theme);
  }, [theme]);
  return (
    <I18nProvider locale={resolveLocale(settings?.language ?? "system", languages)}>
      <HoloProvider dark={dark}>
        <RevealProvider dark={dark}>
          <ToastProvider>
            <SecretsProvider>
              <SessionRoot />
            </SecretsProvider>
          </ToastProvider>
        </RevealProvider>
      </HoloProvider>
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
      <UnlockAnnouncement view={view} />
      <Routes view={view} setSession={setSession} />
    </EventsProvider>
  );
}

/**
 * Says "Mosaic is unlocked." in a live region that outlives the unlock screen (the lock message
 * is on that screen; nothing else tells a screen reader that the vault opened). A start that is
 * already unlocked says nothing.
 */
function UnlockAnnouncement({ view }: { view: SessionView }) {
  const { t } = useI18n();
  const state = view.status === "ready" ? view.session.state : null;
  const [previous, setPrevious] = useState(state);
  const [announce, setAnnounce] = useState(false);
  if (previous !== state) {
    setPrevious(state);
    setAnnounce(previous === "locked" && state === "unlocked");
  }
  return (
    <p className="visually-hidden" role="status">
      {announce ? t("session.unlocked") : ""}
    </p>
  );
}

type SetSession = Parameters<typeof OnboardingFlow>[0]["onDone"];

/** Which screen the session shows, as the reveal stage sees it. */
function targetOf(view: SessionView): RevealTarget {
  if (view.status === "loading") {
    return { kind: "other", key: "loading" };
  }
  if (view.status === "failed") {
    return { kind: "other", key: "failed" };
  }
  const session = view.session;
  switch (session.state) {
    case "noAccount":
      return { kind: "other", key: "onboarding" };
    case "locked":
      return { kind: "locked", session };
    case "unlocked":
      return session.kitVerified ? { kind: "vault" } : { kind: "other", key: "kit" };
  }
}

function Routes({ view, setSession }: { view: SessionView; setSession: SetSession }) {
  return (
    <RevealStage
      target={targetOf(view)}
      renderUnlock={(session) => <UnlockScreen session={session} onUnlocked={setSession} />}
      renderVault={() => <VaultLayout onLocked={setSession} />}
      other={<OtherRoute view={view} setSession={setSession} />}
    />
  );
}

/** The screens that are not part of the lock and unlock reveal. */
function OtherRoute({ view, setSession }: { view: SessionView; setSession: SetSession }) {
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
    case "unlocked":
      return <KitRecoveryFlow onDone={setSession} />;
    case "locked":
      return null;
  }
}
