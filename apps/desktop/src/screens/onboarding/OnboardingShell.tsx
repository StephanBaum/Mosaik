/** The auth frame with the five-step setup progress (D02 to D05). */
import type { ReactNode } from "react";
import { AuthShell } from "../../components/AuthShell.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";

export const STEP_NAMES = ["create", "kit", "verify", "import", "done"] as const;
export const TOTAL_STEPS = STEP_NAMES.length;

export function OnboardingShell({ step, children }: { step: number; children: ReactNode }) {
  const { t } = useI18n();
  const name = STEP_NAMES[step - 1] ?? "done";
  return (
    <AuthShell progress={{ step, total: TOTAL_STEPS, label: t(`onboarding.stepName.${name}`) }}>
      {children}
    </AuthShell>
  );
}
