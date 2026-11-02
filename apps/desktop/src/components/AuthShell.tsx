/**
 * The frame of the screens before the vault (unlock, onboarding, kit recovery, import):
 * a logo header and one centred column, with an optional setup progress bar.
 */
import type { ReactNode } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { Brand } from "./Brand.tsx";
import { HoloSurface } from "./HoloSurface.tsx";

export interface AuthProgress {
  step: number;
  total: number;
  /** The name of the current step. */
  label: string;
}

function Progress({ step, total, label }: AuthProgress) {
  const { t } = useI18n();
  return (
    <div
      className="progress"
      role="progressbar"
      aria-label={t("onboarding.progress")}
      aria-valuemin={1}
      aria-valuemax={total}
      aria-valuenow={step}
      aria-valuetext={t("onboarding.step", { current: step, total })}
    >
      <div className="progress__bar" aria-hidden="true">
        {Array.from({ length: total }, (_, index) =>
          index < step ? (
            <span key={index} className="progress__seg progress__seg--done">
              <HoloSurface className="progress__foil" />
            </span>
          ) : (
            <span key={index} className="progress__seg" />
          ),
        )}
      </div>
      <div className="progress__caption caption" aria-hidden="true">
        <span className="tabular">{t("onboarding.step", { current: step, total })}</span>
        <span>{label}</span>
      </div>
    </div>
  );
}

export function AuthShell({
  progress,
  children,
  center = false,
}: {
  progress?: AuthProgress;
  children: ReactNode;
  /** Centres the column vertically (unlock). */
  center?: boolean;
}) {
  return (
    <div className="auth">
      <header className="auth__header">
        <Brand />
      </header>
      <main id="main" className={center ? "auth__body auth__body--center" : "auth__body"}>
        <div className="auth__column">
          {progress !== undefined && <Progress {...progress} />}
          {children}
        </div>
      </main>
    </div>
  );
}
