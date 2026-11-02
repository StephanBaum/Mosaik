/** A live one-time code: refetched when it expires, hidden on blur and on lock. */
import { useCallback, useEffect, useRef, useState } from "react";
import { useSecrets } from "../../app/SecretsContext.tsx";
import { CopyButton } from "../../components/CopyButton.tsx";
import { CountdownRing } from "../../components/CountdownRing.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { IconButton } from "../../components/IconButton.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { IpcError, ItemRequest, TotpCode as Code } from "../../ipc/contract/index.ts";

const MASKED = "••• •••";
const DEFAULT_PERIOD = 30;

/** Six digits read as 3 + 3; other lengths stay as they are (display only). */
export function groupCode(code: string): string {
  return code.length === 6 ? `${code.slice(0, 3)} ${code.slice(3)}` : code;
}

/** Counts whole seconds while a code with an expiry is shown. */
function useElapsed(running: boolean, reset: unknown): number {
  const [state, setState] = useState({ reset, seconds: 0 });
  useEffect(() => {
    if (!running) {
      return undefined;
    }
    const tick = setInterval(() => {
      setState((current) => ({
        reset,
        seconds: current.reset === reset ? current.seconds + 1 : 1,
      }));
    }, 1000);
    return () => {
      clearInterval(tick);
    };
  }, [running, reset]);
  return state.reset === reset ? state.seconds : 0;
}

export function TotpCode({ item }: { item: ItemRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const secrets = useSecrets();
  const [shown, setShown] = useState(false);
  const [round, setRound] = useState(0);
  const [code, setCode] = useState<Code | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const elapsed = useElapsed(code?.secondsRemaining != null, code);
  const left = code?.secondsRemaining == null ? null : Math.max(code.secondsRemaining - elapsed, 0);

  const hide = useCallback(() => {
    clearTimeout(timer.current);
    setShown(false);
    setCode(null);
  }, []);

  useEffect(() => {
    const unregister = secrets.register(hide);
    return unregister;
  }, [secrets, hide]);

  useEffect(() => {
    if (!shown) {
      return;
    }
    let current = true;
    void ipc.call("totp_code", item).then((result) => {
      if (!current) {
        return;
      }
      if (!result.ok) {
        setError(result.error);
        return;
      }
      setCode(result.value);
      if (result.value.secondsRemaining !== null) {
        // Fetch the next code when this one expires.
        timer.current = setTimeout(() => {
          setRound((n) => n + 1);
        }, result.value.secondsRemaining * 1000);
      }
    });
    return () => {
      current = false;
      clearTimeout(timer.current);
    };
  }, [ipc, item, shown, round]);

  return (
    <div className="row">
      <div className="row__main">
        <span className="row__label">{t("totp.code")}</span>
        <span className="row__value otp-code">{code === null ? MASKED : groupCode(code.code)}</span>
      </div>
      {code !== null && left !== null && (
        <span className="row__meta">
          <CountdownRing remaining={left} total={code.period ?? DEFAULT_PERIOD} />
          {t("totp.secondsShort", { seconds: left })}
        </span>
      )}
      <IconButton
        icon={shown ? "eyeOff" : "eye"}
        label={t(shown ? "totp.hide" : "totp.show")}
        onClick={() => {
          if (shown) {
            hide();
          } else {
            setShown(true);
          }
        }}
      />
      <CopyButton
        source={{ kind: "item", vaultId: item.vaultId, itemId: item.itemId, field: { kind: "totp" } }}
        field={t("totp.code")}
      />
      <ErrorAlert error={error} />
    </div>
  );
}
