/** A labelled input with an optional hint, wired for screen readers. */
import { useId, type InputHTMLAttributes } from "react";

export function TextField({
  label,
  hint,
  mono = false,
  ...input
}: InputHTMLAttributes<HTMLInputElement> & {
  label: string;
  hint?: string | undefined;
  mono?: boolean;
}) {
  const id = useId();
  const hintId = `${id}-hint`;
  return (
    <div className="field">
      <label className="field__label" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        className={mono ? "field__input field__input--mono" : "field__input"}
        aria-describedby={hint === undefined ? undefined : hintId}
        {...input}
      />
      {hint !== undefined && (
        <p id={hintId} className="field__hint">
          {hint}
        </p>
      )}
    </div>
  );
}
