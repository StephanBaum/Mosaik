/**
 * A secret that stays hidden until the user asks (UX_GUIDELINES §8). Revealing asks Rust
 * for the value; it is hidden again after `hideAfterSeconds`, on blur and on lock.
 */
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { useSecrets } from "../app/SecretsContext.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError, IpcResult, RevealedSecret } from "../ipc/contract/index.ts";
import { ErrorAlert } from "./ErrorAlert.tsx";
import { IconButton } from "./IconButton.tsx";

/** What the hidden value looks like. The snapshot painter draws this, never the value. */
const MASK = "••••••••••••••••";

/**
 * The value, or its mask while hidden. Both carry `data-secret` (and `data-mask`), so the
 * reveal's snapshot painter draws the mask and never reads the value (UX_GUIDELINES section 8).
 */
function SecretValue({ value, format }: { value: string | null; format: ((value: string) => string) | undefined }) {
  const { t } = useI18n();
  return value === null ? (
    <span
      className="row__value secret-mask"
      role="img"
      aria-label={t("item.secret.hiddenLabel")}
      data-secret=""
      data-mask={MASK}
    >
      {MASK}
    </span>
  ) : (
    <span className="row__value mono" data-secret="" data-mask={MASK}>
      {format === undefined ? value : format(value)}
    </span>
  );
}

export function SecretField({
  label,
  reveal,
  actions,
  format,
}: {
  label: string;
  reveal: () => Promise<IpcResult<RevealedSecret>>;
  /** How the revealed text is shown (a card number in groups); the copy stays unformatted. */
  format?: ((value: string) => string) | undefined;
  /** Extra controls next to the reveal button, for example a copy button. */
  actions?: ReactNode;
}) {
  const { t } = useI18n();
  const secrets = useSecrets();
  const [value, setValue] = useState<string | null>(null);
  const [seconds, setSeconds] = useState(0);
  const [error, setError] = useState<IpcError | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  /** Bumped by every hide, so an answer that arrives after a hide is dropped. */
  const generation = useRef(0);

  const hide = useCallback(() => {
    generation.current += 1;
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
    const asked = generation.current;
    const result = await reveal();
    // Hidden while the answer was on its way (blur, lock, timeout): never paint it.
    if (generation.current !== asked) {
      return;
    }
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
    <div className="row">
      <div className="row__main">
        <span className="row__label">{label}</span>
        <SecretValue value={value} format={format} />
      </div>
      <IconButton
        icon={value === null ? "eye" : "eyeOff"}
        label={t(value === null ? "item.secret.revealAction" : "item.secret.hideAction", {
          field: label,
        })}
        onClick={() => {
          if (value === null) {
            void show();
          } else {
            hide();
          }
        }}
      />
      {actions}
      <p className="visually-hidden" role="status">
        {value === null ? "" : t("item.secret.revealed", { field: label, seconds })}
      </p>
      <ErrorAlert error={error} />
    </div>
  );
}
