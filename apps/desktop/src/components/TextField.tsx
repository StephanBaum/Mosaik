/** A labelled input with an optional hint and error, wired for screen readers. */
import { useId, type InputHTMLAttributes } from "react";
import { Icon } from "./icons.tsx";

export function describedBy(...ids: (string | undefined)[]): string | undefined {
  const joined = ids.filter(Boolean).join(" ");
  return joined === "" ? undefined : joined;
}

export function FieldError({ id, message }: { id: string; message: string }) {
  return (
    <p id={id} className="field__error" role="alert">
      <Icon name="alert" size="sm" />
      {message}
    </p>
  );
}

export function TextField({
  label,
  hint,
  error,
  mono = false,
  size = "md",
  className,
  ...input
}: Omit<InputHTMLAttributes<HTMLInputElement>, "size"> & {
  label: string;
  hint?: string | undefined;
  error?: string | undefined;
  mono?: boolean;
  size?: "md" | "lg";
}) {
  const id = useId();
  const hintId = `${id}-hint`;
  const errorId = `${id}-error`;
  return (
    <div className={size === "lg" ? "field field--lg" : "field"}>
      <label className="field__label" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        className={["field__input", mono ? "field__input--mono" : "", className].filter(Boolean).join(" ")}
        aria-describedby={describedBy(
          hint === undefined ? undefined : hintId,
          error === undefined ? undefined : errorId,
        )}
        aria-invalid={error === undefined ? undefined : true}
        {...input}
      />
      {hint !== undefined && (
        <p id={hintId} className="field__hint">
          {hint}
        </p>
      )}
      {error !== undefined && <FieldError id={errorId} message={error} />}
    </div>
  );
}
