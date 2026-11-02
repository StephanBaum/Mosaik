/** A labelled native select. */
import { useId, type SelectHTMLAttributes } from "react";

export interface Option {
  value: string;
  label: string;
}

export function Select({
  label,
  options,
  inline = false,
  ...select
}: SelectHTMLAttributes<HTMLSelectElement> & {
  label: string;
  options: readonly Option[];
  /** Label and select on one settings row. */
  inline?: boolean;
}) {
  const id = useId();
  return (
    <div className={inline ? "setting-row" : "field"}>
      <label className={inline ? "setting-row__label" : "field__label"} htmlFor={id}>
        {label}
      </label>
      <select
        id={id}
        className={inline ? "field__input setting-row__control" : "field__input"}
        {...select}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}
