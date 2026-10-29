/** A live one-time code: refetched when it expires, hidden on blur and on lock. */
import { useCallback, useEffect, useRef, useState } from "react";
import { useSecrets } from "../../app/SecretsContext.tsx";
import { Button } from "../../components/Button.tsx";
import { CopyButton } from "../../components/CopyButton.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { IpcError, ItemRequest, TotpCode as Code } from "../../ipc/contract/index.ts";

export function TotpCode({ item }: { item: ItemRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const secrets = useSecrets();
  const [shown, setShown] = useState(false);
  const [round, setRound] = useState(0);
  const [code, setCode] = useState<Code | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

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
    <div className="secret">
      <span className="secret__label">{t("totp.code")}</span>
      <div className="secret__row">
        <span className="secret__value mono">{code === null ? "••• •••" : code.code}</span>
        <Button
          variant="ghost"
          onClick={() => {
            if (shown) {
              hide();
            } else {
              setShown(true);
            }
          }}
        >
          {t(shown ? "totp.hide" : "totp.show")}
        </Button>
        <CopyButton
          source={{ kind: "item", vaultId: item.vaultId, itemId: item.itemId, field: { kind: "totp" } }}
          field={t("totp.code")}
        />
      </div>
      {code?.secondsRemaining != null && (
        <p className="field__hint">{t("totp.expires", { seconds: code.secondsRemaining })}</p>
      )}
      <ErrorAlert error={error} />
    </div>
  );
}
