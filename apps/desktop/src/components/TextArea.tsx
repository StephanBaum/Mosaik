/** A labelled multi-line input with an optional hint, wired for screen readers (mirrors `TextField`). */
import { useId, type TextareaHTMLAttributes } from "react";

export function TextArea({
  label,
  hint,
  ...input
}: TextareaHTMLAttributes<HTMLTextAreaElement> & {
  label: string;
  hint?: string | undefined;
}) {
  const id = useId();
  const hintId = `${id}-hint`;
  return (
    <div className="field">
      <label className="field__label" htmlFor={id}>
        {label}
      </label>
      <textarea
        id={id}
        className="field__input field__input--multiline"
        rows={4}
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
