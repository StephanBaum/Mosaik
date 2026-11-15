/**
 * Import (D05): pick where the passwords are now, follow that product's export steps, choose the
 * file (Rust opens the dialog and reads it), preview with duplicate hints and what will not come
 * across, pick the target vault, commit, then clean up the plaintext export. The export's
 * contents never reach this component: only titles, counts and warnings.
 */
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { OnboardingShell } from "../onboarding/OnboardingShell.tsx";
import { CleanupStep } from "./CleanupStep.tsx";
import { FileStep, PreviewStep } from "./ImportSteps.tsx";
import { SourceGrid } from "./SourceGrid.tsx";
import { SourceSteps } from "./SourceSteps.tsx";
import { sourceById } from "./sources.ts";
import { useImportFlow } from "./useImportFlow.ts";

export function ImportFlow({ onClose, embedded = false }: { onClose: () => void; embedded?: boolean }) {
  const { t } = useI18n();
  const flow = useImportFlow(onClose);
  const { step, setStep, setError } = flow;
  const body = (
    <div className="stack">
      <h1 className="display">{t("import.flow.title")}</h1>
      {step.name === "source" && (
        <SourceGrid
          onPick={(source) => {
            setError(null);
            setStep({ name: "steps", source });
          }}
        />
      )}
      {step.name === "steps" && (
        <SourceSteps
          source={sourceById(step.source)}
          busy={flow.busy}
          onChoose={() => void flow.choose(step.source)}
          onBack={() => {
            setError(null);
            setStep({ name: "source" });
          }}
        />
      )}
      {step.name === "file" && (
        <FileStep
          source={sourceById(step.source)}
          fileName={step.fileName}
          detected={step.detected}
          busy={flow.busy}
          onPreview={(password) => void flow.preview(step, password)}
        />
      )}
      {step.name === "preview" && (
        <PreviewStep
          source={sourceById(step.source)}
          preview={step.preview}
          vaults={flow.vaults}
          onCommit={(target, skip) => void flow.commit(step, step.preview.previewId, target, skip)}
        />
      )}
      {step.name === "cleanup" && (
        <CleanupStep
          source={sourceById(step.source)}
          token={step.token}
          fileName={step.fileName}
          synced={step.detected.syncedFolder}
          result={step.result}
          onDone={onClose}
        />
      )}
      <ErrorAlert error={flow.error} />
      {step.name !== "cleanup" && (
        <Button variant="ghost" onClick={() => void flow.cancel()}>
          {t("import.flow.cancel")}
        </Button>
      )}
    </div>
  );
  return <Frame embedded={embedded}>{body}</Frame>;
}

function Frame({ embedded, children }: { embedded: boolean; children: React.ReactNode }) {
  const { t } = useI18n();
  return embedded ? (
    <section className="import-embedded" aria-label={t("import.flow.title")}>
      {children}
    </section>
  ) : (
    <OnboardingShell step={4}>{children}</OnboardingShell>
  );
}
