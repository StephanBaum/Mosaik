/**
 * A typed password with a show/hide toggle. For passwords the user types only (master
 * password, editor); stored secrets stay in `SecretField`, which reveals through Rust.
 */
import { useId, useState, type InputHTMLAttributes } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { IconButton } from "./IconButton.tsx";
import { Icon } from "./icons.tsx";
import { describedBy, FieldError } from "./TextField.tsx";

function toggleName(
  t: ReturnType<typeof useI18n>["t"],
  kind: "field" | "password",
  field: string,
  shown: boolean,
): string {
  if (kind === "password") {
    return shown ? t("action.hidePassword") : t("action.showPassword");
  }
  return shown ? t("item.secret.hideAction", { field }) : t("item.secret.revealAction", { field });
}

export function PasswordField({
  label,
  hint,
  hintTone,
  error,
  size = "md",
  toggleLabel = "field",
  ...input
}: Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> & {
  label: string;
  hint?: string | undefined;
  /** A hint that something looks wrong but does not block saving: warning icon, announced politely. */
  hintTone?: "warning" | undefined;
  error?: string | undefined;
  size?: "md" | "lg";
  /**
   * What the show/hide button is called: after the field ("Reveal Card number", so several
   * toggles in one form differ) or "Show password" for a master-password screen.
   */
  toggleLabel?: "field" | "password";
}) {
  const { t } = useI18n();
  const id = useId();
  const [shown, setShown] = useState(false);
  const hintId = `${id}-hint`;
  const errorId = `${id}-error`;
  return (
    <div className={size === "lg" ? "field field--lg" : "field"}>
      <label className="field__label" htmlFor={id}>
        {label}
      </label>
      <div className="input-group">
        <input
          id={id}
          className="field__input"
          type={shown ? "text" : "password"}
          aria-describedby={describedBy(
            hint === undefined ? undefined : hintId,
            error === undefined ? undefined : errorId,
          )}
          aria-invalid={error === undefined ? undefined : true}
          {...input}
        />
        <IconButton
          icon={shown ? "eyeOff" : "eye"}
          label={toggleName(t, toggleLabel, label, shown)}
          aria-controls={id}
          pressed={shown}
          onClick={() => {
            setShown((current) => !current);
          }}
        />
      </div>
      {hint !== undefined &&
        (hintTone === "warning" ? (
          <p id={hintId} className="field__hint field__hint--warning" role="status">
            <Icon name="alert" size="sm" />
            {hint}
          </p>
        ) : (
          <p id={hintId} className="field__hint">
            {hint}
          </p>
        ))}
      {error !== undefined && <FieldError id={errorId} message={error} />}
    </div>
  );
}
