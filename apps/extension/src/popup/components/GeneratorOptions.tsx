/** The generator's controls (E03): the type switch, the length slider and the character checkboxes. Limits are the desktop's. */
import { useId, type KeyboardEvent } from "react";
import { useI18n } from "../i18n.tsx";

export interface PasswordOptions {
  kind: "password";
  length: number;
  lowercase: boolean;
  uppercase: boolean;
  digits: boolean;
  symbols: boolean;
  excludeAmbiguous: boolean;
}

export interface PassphraseOptions {
  kind: "passphrase";
  words: number;
  separator: string;
  capitalize: boolean;
  appendDigit: boolean;
}

export const DEFAULT_PASSWORD: PasswordOptions = {
  kind: "password",
  length: 20,
  lowercase: true,
  uppercase: true,
  digits: true,
  symbols: true,
  excludeAmbiguous: true,
};

export const DEFAULT_PASSPHRASE: PassphraseOptions = {
  kind: "passphrase",
  words: 5,
  separator: "-",
  capitalize: false,
  appendDigit: false,
};

export type Mode = "random" | "passphrase";
const MODES: readonly Mode[] = ["random", "passphrase"];
export const LENGTH_RANGE = { min: 8, max: 64 } as const;
export const WORDS_RANGE = { min: 3, max: 10 } as const;

export function TypeSwitch({ mode, onMode }: { mode: Mode; onMode: (mode: Mode) => void }) {
  const { t } = useI18n();
  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
      event.preventDefault();
      const next = MODES[(MODES.indexOf(mode) + 1) % MODES.length] ?? "random";
      onMode(next);
      event.currentTarget.querySelector<HTMLElement>(`[data-mode="${next}"]`)?.focus();
    }
  }
  return (
    <div className="segmented" role="radiogroup" aria-label={t("ext.generator.type")} onKeyDown={onKeyDown}>
      {MODES.map((each) => (
        <button
          key={each}
          type="button"
          role="radio"
          data-mode={each}
          aria-checked={each === mode}
          tabIndex={each === mode ? 0 : -1}
          className="segmented__option"
          onClick={() => {
            onMode(each);
          }}
        >
          {t(each === "random" ? "ext.generator.random" : "ext.generator.passphrase")}
        </button>
      ))}
    </div>
  );
}

function Check({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void }) {
  return (
    <label className="check">
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => {
          onChange(event.target.checked);
        }}
      />
      <span>{label}</span>
    </label>
  );
}

function Slider({
  label,
  valueText,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  valueText: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  const id = useId();
  return (
    <div className="slider">
      <div className="slider__head">
        <label htmlFor={id} className="label">
          {label}
        </label>
        <output htmlFor={id} className="slider__value">
          {valueText}
        </output>
      </div>
      <input
        id={id}
        className="range"
        type="range"
        min={min}
        max={max}
        value={value}
        aria-valuetext={valueText}
        onChange={(event) => {
          onChange(Number(event.target.value));
        }}
      />
    </div>
  );
}

const FLAGS = ["uppercase", "lowercase", "digits", "symbols"] as const;

export function RandomControls({ value, onChange }: { value: PasswordOptions; onChange: (next: PasswordOptions) => void }) {
  const { t } = useI18n();
  return (
    <>
      <Slider
        label={t("generator.length")}
        valueText={t("generator.lengthValue", { length: value.length })}
        value={value.length}
        {...LENGTH_RANGE}
        onChange={(length) => {
          onChange({ ...value, length });
        }}
      />
      <fieldset className="fieldset">
        <legend className="label">{t("ext.generator.include")}</legend>
        {FLAGS.map((flag) => (
          <Check
            key={flag}
            label={t(`generator.${flag}`)}
            checked={value[flag]}
            onChange={(checked) => {
              onChange({ ...value, [flag]: checked });
            }}
          />
        ))}
        <Check
          label={t("generator.excludeAmbiguous")}
          checked={value.excludeAmbiguous}
          onChange={(excludeAmbiguous) => {
            onChange({ ...value, excludeAmbiguous });
          }}
        />
      </fieldset>
    </>
  );
}

export function PassphraseControls({ value, onChange }: { value: PassphraseOptions; onChange: (next: PassphraseOptions) => void }) {
  const { t } = useI18n();
  return (
    <>
      <Slider
        label={t("generator.words")}
        valueText={t("generator.wordsValue", { length: value.words })}
        value={value.words}
        {...WORDS_RANGE}
        onChange={(words) => {
          onChange({ ...value, words });
        }}
      />
      <fieldset className="fieldset">
        <legend className="label">{t("ext.generator.include")}</legend>
        <Check
          label={t("generator.capitalize")}
          checked={value.capitalize}
          onChange={(capitalize) => {
            onChange({ ...value, capitalize });
          }}
        />
        <Check
          label={t("generator.appendDigit")}
          checked={value.appendDigit}
          onChange={(appendDigit) => {
            onChange({ ...value, appendDigit });
          }}
        />
      </fieldset>
    </>
  );
}
