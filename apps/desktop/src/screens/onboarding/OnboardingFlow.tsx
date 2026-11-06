/**
 * Onboarding (UX_GUIDELINES §8, D02 to D06): create the account, save and verify the
 * Emergency Kit, import (skippable), the optional browser extension (D06) and into the vault.
 * The flow holds the Secret Key only until the kit is verified.
 */
import { useReducer } from "react";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Button } from "../../components/Button.tsx";
import { Icon } from "../../components/icons.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { SessionState } from "../../ipc/contract/index.ts";
import { ImportFlow } from "../import/ImportFlow.tsx";
import { BrowserStep } from "./BrowserStep.tsx";
import { flowReducer, initialFlow } from "./flow.ts";
import { KitStep } from "./KitStep.tsx";
import { VerifyStep } from "./VerifyStep.tsx";
import { OnboardingShell } from "./OnboardingShell.tsx";
import { WelcomeStep } from "./WelcomeStep.tsx";

export function OnboardingFlow({ onDone }: { onDone: (session: SessionState) => void }) {
  const [state, dispatch] = useReducer(flowReducer, initialFlow);

  if (state.session === null) {
    return (
      <WelcomeStep
        onCreated={(account) => {
          dispatch({ type: "created", account });
        }}
      />
    );
  }
  const { session } = state;
  switch (state.step) {
    case "kit":
      return state.kit === null ? null : (
        <KitStep
          kit={state.kit}
          onContinue={() => {
            dispatch({ type: "kitSaved" });
          }}
        />
      );
    case "verify":
      return (
        <VerifyStep
          onVerified={() => {
            dispatch({ type: "verified" });
          }}
        />
      );
    case "import":
      return (
        <ImportOffer
          onImport={() => {
            dispatch({ type: "importStart" });
          }}
          onSkip={() => {
            dispatch({ type: "skip" });
          }}
        />
      );
    case "importing":
      return (
        <ImportFlow
          onClose={() => {
            dispatch({ type: "done" });
          }}
        />
      );
    case "browser":
      return (
        <BrowserStep
          onDone={() => {
            onDone(verified(session));
          }}
        />
      );
  }
}

function ImportOffer({ onImport, onSkip }: { onImport: () => void; onSkip: () => void }) {
  const { t } = useI18n();
  return (
    <OnboardingShell step={4}>
      <div className="auth__main">
        <div className="auth__intro">
          <h1 className="display">{t("onboarding.import.title")}</h1>
          <p className="lede">{t("onboarding.import.body")}</p>
        </div>
        <div className="auth__actions">
          <HoloButton size="lg" block onClick={onImport}>
            <Icon name="download" />
            {t("onboarding.import.start")}
          </HoloButton>
          <Button variant="ghost" size="lg" block className="button--muted" onClick={onSkip}>
            {t("onboarding.import.skip")}
          </Button>
        </div>
      </div>
    </OnboardingShell>
  );
}

/** The account was verified during this flow. */
function verified(session: SessionState): SessionState {
  return session.state === "unlocked" ? { ...session, kitVerified: true } : session;
}
