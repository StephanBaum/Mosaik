/** A labelled native select. */
import { useId, type SelectHTMLAttributes } from "react";

export interface Option {
  value: string;
  label: string;
}

export function Select({
  label,
  options,
  ...select
}: SelectHTMLAttributes<HTMLSelectElement> & { label: string; options: readonly Option[] }) {
  const id = useId();
  return (
    <div className="field">
      <label className="field__label" htmlFor={id}>
        {label}
      </label>
      <select id={id} className="field__input" {...select}>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}
