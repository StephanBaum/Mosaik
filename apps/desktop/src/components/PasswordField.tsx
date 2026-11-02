/**
 * A typed password with a show/hide toggle. For passwords the user types only (master
 * password, editor); stored secrets stay in `SecretField`, which reveals through Rust.
 */
import { useId, useState, type InputHTMLAttributes } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { IconButton } from "./IconButton.tsx";
import { describedBy, FieldError } from "./TextField.tsx";

export function PasswordField({
  label,
  hint,
  error,
  size = "md",
  ...input
}: Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> & {
  label: string;
  hint?: string | undefined;
  error?: string | undefined;
  size?: "md" | "lg";
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
          label={shown ? t("action.hidePassword") : t("action.showPassword")}
          pressed={shown}
          onClick={() => {
            setShown((current) => !current);
          }}
        />
      </div>
      {hint !== undefined && (
        <p id={hintId} className="field__hint">
          {hint}
        </p>
      )}
      {error !== undefined && <FieldError id={errorId} message={error} />}
    </div>
  );
}
