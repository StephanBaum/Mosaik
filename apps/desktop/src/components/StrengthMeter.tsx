/** Password strength as text and a bar, never color alone (UX_GUIDELINES §7). */
import { useI18n } from "../i18n/I18nProvider.tsx";
import { strengths, type Strength } from "../ipc/contract/index.ts";

export function StrengthMeter({ strength, bits }: { strength: Strength; bits: number }) {
  const { t } = useI18n();
  const level = strengths.indexOf(strength) + 1;
  const label = t(`generator.strength.${strength}`);
  return (
    <div className="strength">
      <div className="strength__bar" aria-hidden="true">
        {strengths.map((name, index) => (
          <span key={name} className={index < level ? "strength__step is-on" : "strength__step"} />
        ))}
      </div>
      <p className="body">
        {t("generator.strengthLabel", { strength: label })}
        <span className="muted"> · {t("generator.entropy", { bits: Math.round(bits) })}</span>
      </p>
    </div>
  );
}
