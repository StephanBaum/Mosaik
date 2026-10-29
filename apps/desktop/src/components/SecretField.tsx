/**
 * A secret that stays hidden until the user asks (UX_GUIDELINES §8). Revealing asks Rust
 * for the value; it is hidden again after `hideAfterSeconds`, on blur and on lock.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { useSecrets } from "../app/SecretsContext.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError, IpcResult, RevealedSecret } from "../ipc/contract/index.ts";
import { Button } from "./Button.tsx";
import { ErrorAlert } from "./ErrorAlert.tsx";

export function SecretField({
  label,
  reveal,
  actions,
}: {
  label: string;
  reveal: () => Promise<IpcResult<RevealedSecret>>;
  /** Extra controls next to the reveal button, for example a copy button. */
  actions?: React.ReactNode;
}) {
  const { t } = useI18n();
  const secrets = useSecrets();
  const [value, setValue] = useState<string | null>(null);
  const [seconds, setSeconds] = useState(0);
  const [error, setError] = useState<IpcError | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const hide = useCallback(() => {
    clearTimeout(timer.current);
    setValue(null);
  }, []);

  useEffect(() => {
    const unregister = secrets.register(hide);
    return () => {
      unregister();
      clearTimeout(timer.current);
    };
  }, [secrets, hide]);

  async function show() {
    setError(null);
    const result = await reveal();
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setValue(result.value.value);
    setSeconds(result.value.hideAfterSeconds);
    clearTimeout(timer.current);
    timer.current = setTimeout(hide, result.value.hideAfterSeconds * 1000);
  }

  return (
    <div className="secret">
      <span className="secret__label">{label}</span>
      <div className="secret__row">
        {value === null ? (
          <span className="secret__value mono" role="img" aria-label={t("item.secret.hiddenLabel")}>
            ••••••••••••
          </span>
        ) : (
          <span className="secret__value mono">{value}</span>
        )}
        <Button
          variant="ghost"
          aria-label={t(value === null ? "item.secret.revealAction" : "item.secret.hideAction", {
            field: label,
          })}
          onClick={() => {
            if (value === null) {
              void show();
            } else {
              hide();
            }
          }}
        >
          {t(value === null ? "action.reveal" : "action.hide")}
        </Button>
        {actions}
      </div>
      <p className="visually-hidden" role="status">
        {value === null ? "" : t("item.secret.revealed", { field: label, seconds })}
      </p>
      <ErrorAlert error={error} />
    </div>
  );
}
