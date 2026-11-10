/**
 * The second-factor step at connect and join (M9). It asks Rust which methods the waiting login
 * can use: an authenticator-app code (the `children`, the existing code form) and/or a security
 * key, which runs in the system browser ([`SecurityKeyStep`]). An account with a security key
 * only shows the key and no code field. If the methods cannot be read, the code form stays (the
 * way it was before security keys).
 */
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, LoginMethods } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { SecurityKeyStep, type StepView } from "./SecurityKeyStep.tsx";

/** The methods of the waiting login; `null` while they are read. */
function useLoginMethods(): LoginMethods | null {
  const ipc = useIpc();
  const [methods, setMethods] = useState<LoginMethods | null>(null);
  useEffect(() => {
    let current = true;
    void ipc.call("sync_login_methods", null).then((result) => {
      if (current) {
        setMethods(result.ok ? result.value : { totp: true, webauthn: false });
      }
    });
    return () => {
      current = false;
    };
  }, [ipc]);
  return methods;
}

/** "Use security key": the heading (when there is no code form) and the button that starts it. */
function KeyChoice({
  keyOnly,
  large,
  starting,
  onStart,
}: {
  keyOnly: boolean;
  large: boolean;
  starting: boolean;
  onStart: () => void;
}) {
  const { t } = useI18n();
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, []);
  return (
    <section className="stack">
      {keyOnly && (
        <>
          <h4 className="body" ref={heading} tabIndex={-1}>
            {t("settings.sync.key.step.keyOnlyTitle")}
          </h4>
          <p className="field__hint">{t("settings.sync.key.step.keyOnlyBody")}</p>
        </>
      )}
      <Button {...(large ? { size: "lg", block: true } : {})} onClick={onStart} disabled={starting}>
        {t("settings.sync.key.step.useKey")}
      </Button>
    </section>
  );
}

export function SecondFactorStep({
  join,
  large = false,
  children,
  onSignedIn,
  onCancel,
}: {
  /** The waiting login is a join (a new device without an account). */
  join: boolean;
  /** Bigger buttons, as the onboarding screens have them. */
  large?: boolean;
  /** The authenticator-app code form of the screen. */
  children: ReactNode;
  /** The security key finished the sign-in (Rust ran the connect or join). */
  onSignedIn: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const methods = useLoginMethods();
  const [view, setView] = useState<StepView | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [starting, setStarting] = useState(false);

  async function begin() {
    setStarting(true);
    setError(null);
    const command = join ? "join_hosted_webauthn_begin" : "sync_hosted_webauthn_begin";
    const result = await ipc.call(command, null);
    setStarting(false);
    if (result.ok) {
      setView({ step: "login", ...result.value });
    } else {
      setError(result.error);
    }
  }

  if (methods === null) {
    return (
      <p className="status" role="status">
        {t("settings.sync.key.step.methods")}
      </p>
    );
  }
  if (view !== null) {
    return (
      <SecurityKeyStep
        view={view}
        poll={() => ipc.call("sync_webauthn_status", null)}
        onDone={onSignedIn}
        onCancel={() => {
          setView(null);
        }}
        onRetry={() => {
          setView(null);
          void begin();
        }}
      />
    );
  }
  return (
    <div className="stack">
      {methods.webauthn && (
        <KeyChoice
          keyOnly={!methods.totp}
          large={large}
          starting={starting}
          onStart={() => {
            void begin();
          }}
        />
      )}
      {methods.totp && methods.webauthn && (
        <p className="caption">{t("settings.sync.key.step.or")}</p>
      )}
      {methods.totp && children}
      <ErrorAlert error={error} />
      {!methods.totp && (
        <Button variant="ghost" {...(large ? { size: "lg", block: true } : {})} onClick={onCancel}>
          {t("action.cancel")}
        </Button>
      )}
    </div>
  );
}
