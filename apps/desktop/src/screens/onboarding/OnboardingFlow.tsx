/**
 * Onboarding (UX_GUIDELINES §8, D02 to D05): create the account, save and verify the
 * Emergency Kit, import (skippable), done. The flow holds the Secret Key only until the
 * kit is verified. Step D06 (install the extension) comes with M4.
 */
import { useReducer } from "react";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Button } from "../../components/Button.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { SessionState } from "../../ipc/contract/index.ts";
import { ImportFlow } from "../import/ImportFlow.tsx";
import { flowReducer, initialFlow } from "./flow.ts";
import { KitStep } from "./KitStep.tsx";
import { VerifyStep } from "./VerifyStep.tsx";
import { TOTAL_STEPS, WelcomeStep } from "./WelcomeStep.tsx";

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
    case "done":
      return <Done onOpen={() => { onDone(verified(session)); }} />;
  }
}

function ImportOffer({ onImport, onSkip }: { onImport: () => void; onSkip: () => void }) {
  const { t } = useI18n();
  return (
    <main id="main" className="screen screen--centered">
      <div className="panel">
        <p className="eyebrow">{t("onboarding.step", { current: 4, total: TOTAL_STEPS })}</p>
        <h1 className="title">{t("onboarding.import.title")}</h1>
        <p className="body muted">{t("onboarding.import.body")}</p>
        <HoloButton onClick={onImport}>{t("onboarding.import.start")}</HoloButton>
        <Button onClick={onSkip}>{t("onboarding.import.skip")}</Button>
      </div>
    </main>
  );
}

function Done({ onOpen }: { onOpen: () => void }) {
  const { t } = useI18n();
  return (
    <main id="main" className="screen screen--centered">
      <div className="panel">
        <p className="eyebrow">{t("onboarding.step", { current: 5, total: TOTAL_STEPS })}</p>
        <h1 className="title">{t("onboarding.done.title")}</h1>
        <p className="body muted">{t("onboarding.done.body")}</p>
        <HoloButton onClick={onOpen}>{t("onboarding.done.open")}</HoloButton>
      </div>
    </main>
  );
}

/** The account was verified during this flow. */
function verified(session: SessionState): SessionState {
  return session.state === "unlocked" ? { ...session, kitVerified: true } : session;
}
