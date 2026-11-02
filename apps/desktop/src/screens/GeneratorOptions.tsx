/** The generator's controls: mode tabs, the length slider and the character checkboxes. */
import { useId, type KeyboardEvent } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { GenerateRequest } from "../ipc/contract/index.ts";

export type Options = Extract<GenerateRequest, { kind: "password" }>;
export type Phrase = Extract<GenerateRequest, { kind: "passphrase" }>;
export type Mode = "password" | "passphrase";

const MODES: readonly Mode[] = ["password", "passphrase"];
type Flag = "lowercase" | "uppercase" | "digits" | "symbols" | "excludeAmbiguous";
const CHARACTER_FLAGS: readonly Flag[] = ["lowercase", "uppercase", "digits", "symbols"];

export const panelId = (tabsId: string) => `${tabsId}-panel`;

export function ModeTabs({ id, mode, onMode }: { id: string; mode: Mode; onMode: (mode: Mode) => void }) {
  const { t } = useI18n();
  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
      event.preventDefault();
      const next = MODES[(MODES.indexOf(mode) + 1) % MODES.length] ?? "password";
      onMode(next);
      document.getElementById(`${id}-${next}`)?.focus();
    }
  }
  return (
    <div className="segmented" role="tablist" aria-label={t("generator.mode")} onKeyDown={onKeyDown}>
      {MODES.map((each) => (
        <button
          key={each}
          id={`${id}-${each}`}
          type="button"
          role="tab"
          className="segmented__tab"
          aria-selected={each === mode}
          aria-controls={panelId(id)}
          tabIndex={each === mode ? 0 : -1}
          onClick={() => {
            onMode(each);
          }}
        >
          {t(`generator.mode.${each}`)}
        </button>
      ))}
    </div>
  );
}

function Check({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="check">
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => {
          onChange(event.target.checked);
        }}
      />
      {label}
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
        <label htmlFor={id} className="slider__label">
          {label}
        </label>
        <output htmlFor={id} className="slider__value tabular">
          {value}
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
      <div className="slider__scale caption tabular">
        <span>{min}</span>
        <span>{max}</span>
      </div>
    </div>
  );
}

export function PasswordOptions({ value, onChange }: { value: Options; onChange: (next: Options) => void }) {
  const { t } = useI18n();
  return (
    <>
      <Slider
        label={t("generator.length")}
        valueText={t("generator.lengthValue", { length: value.length })}
        value={value.length}
        min={8}
        max={64}
        onChange={(length) => {
          onChange({ ...value, length });
        }}
      />
      <fieldset className="fieldset">
        <legend className="fieldset__legend">{t("generator.characters")}</legend>
        <div className="check-grid">
          {CHARACTER_FLAGS.map((flag) => (
            <Check
              key={flag}
              label={t(`generator.${flag}`)}
              checked={value[flag]}
              onChange={(checked) => {
                onChange({ ...value, [flag]: checked });
              }}
            />
          ))}
        </div>
        <Check
          label={t("generator.excludeAmbiguous")}
          checked={value.excludeAmbiguous}
          onChange={(checked) => {
            onChange({ ...value, excludeAmbiguous: checked });
          }}
        />
      </fieldset>
    </>
  );
}

export function PassphraseOptions({ value, onChange }: { value: Phrase; onChange: (next: Phrase) => void }) {
  const { t } = useI18n();
  return (
    <>
      <Slider
        label={t("generator.words")}
        valueText={t("generator.wordsValue", { length: value.words })}
        value={value.words}
        min={3}
        max={10}
        onChange={(words) => {
          onChange({ ...value, words });
        }}
      />
      <div className="check-grid">
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
      </div>
    </>
  );
}
