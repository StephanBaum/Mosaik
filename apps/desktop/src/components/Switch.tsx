/** An on/off switch on a native checkbox, so keyboard and screen readers just work. */
import { useId, type InputHTMLAttributes } from "react";

export function Switch({
  label,
  hint,
  ...input
}: Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & { label: string; hint?: string }) {
  const id = useId();
  return (
    <div className="switch">
      <input
        id={id}
        type="checkbox"
        role="switch"
        className="switch__input"
        aria-describedby={hint === undefined ? undefined : `${id}-hint`}
        {...input}
      />
      <label htmlFor={id} className="switch__label">
        {label}
      </label>
      {hint !== undefined && (
        <p id={`${id}-hint`} className="field__hint">
          {hint}
        </p>
      )}
    </div>
  );
}
