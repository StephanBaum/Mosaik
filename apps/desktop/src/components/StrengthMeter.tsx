/** Password strength as text and four bars, never color alone (UX_GUIDELINES §7). */
import { useI18n } from "../i18n/I18nProvider.tsx";
import { strengths, type Strength } from "../ipc/contract/index.ts";

/** Four bars for five strengths: "very weak" lights none, "very strong" lights all four. */
const BARS = [0, 1, 2, 3] as const;

export function StrengthMeter({ strength, bits }: { strength: Strength; bits: number }) {
  const { t } = useI18n();
  const level = strengths.indexOf(strength);
  return (
    <div className="strength">
      <div className="strength__bar" aria-hidden="true">
        {BARS.map((bar) => (
          <span key={bar} className={bar < level ? "strength__step is-on" : "strength__step"} />
        ))}
      </div>
      <span className="strength__label">
        {t("generator.strengthShort", { strength: t(`generator.strength.${strength}`), bits: Math.round(bits) })}
      </span>
    </div>
  );
}
