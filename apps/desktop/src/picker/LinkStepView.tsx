/**
 * The link step: which program, which login, what it takes (ADR-0164). One button, disabled for
 * the first second after the step appears and after each confirmation (Rust enforces the same
 * times). An unsigned program takes two confirmations.
 */
import { useEffect, useState } from "react";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError, LinkStep, PickerState, ProgramView } from "../ipc/contract/index.ts";
import type { PickerIpc } from "./ipc.ts";

/** How long the button stays disabled (the Rust guard is the same one second). */
export const CONFIRM_GUARD_MS = 1_000;

function ProgramLine({ program }: { program: ProgramView }) {
  const { t } = useI18n();
  const text =
    program.kind === "package"
      ? program.signer === null
        ? t("programFill.link.packagedNoPublisher", { name: program.name })
        : t("programFill.link.packaged", { name: program.name, publisher: program.signer })
      : program.kind === "signed"
        ? t("programFill.link.signed", { exe: program.exe, signer: program.signer ?? "" })
        : t("programFill.link.unsigned", { exe: program.exe, path: program.path ?? "" });
  return <p className="picker__text picker__program">{text}</p>;
}

function confirmKey(step: LinkStep) {
  if (step.confirmed > 0) {
    return "programFill.link.confirmAgain";
  }
  if (step.confirmations > 1) {
    return "programFill.link.confirmUnsigned";
  }
  return step.submit ? "programFill.link.confirm" : "programFill.link.confirmFill";
}

export function LinkStepView({
  state,
  step,
  ipc,
}: {
  state: PickerState;
  step: LinkStep;
  ipc: PickerIpc;
}) {
  const { t } = useI18n();
  const [error, setError] = useState<IpcError | null>(null);
  // The guard starts again with every step and every confirmation: the button is ready once
  // the timer of the current stage ran out.
  const [retry, setRetry] = useState(0);
  const stage = [step.login.itemId, String(step.confirmed), String(retry)].join(":");
  const [readyStage, setReadyStage] = useState("");
  const ready = readyStage === stage;
  const [pending, setPending] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => {
      setReadyStage(stage);
    }, CONFIRM_GUARD_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [stage]);

  const confirm = async (): Promise<void> => {
    setPending(true);
    const result = await ipc.call("picker_link_confirm", { generation: state.generation });
    setPending(false);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setError(null);
    if (result.value.kind === "tooSoon") {
      // The step stays; the guard starts again.
      setRetry((count) => count + 1);
    }
  };

  return (
    <section className="picker picker--link" aria-label={t("programFill.list.link")}>
      <ProgramLine program={step.program} />
      <p className="picker__text">{t("programFill.link.login", { title: step.login.title })}</p>
      {step.program.kind === "unsigned" ? (
        <p className="picker__note">{t("programFill.link.unsignedNote")}</p>
      ) : null}
      {step.lookalikeOf === null ? null : (
        <p className="picker__warning" role="alert">
          {t("programFill.link.lookalike", { similar: step.lookalikeOf })}
        </p>
      )}
      {step.hashChange === null ? null : (
        <p className="picker__note picker__hash">
          {t("programFill.link.hashChange", {
            old: step.hashChange.oldPrefix,
            new: step.hashChange.newPrefix,
          })}
        </p>
      )}
      <ErrorAlert error={error} />
      <div className="picker__actions">
        <Button size="sm" disabled={!ready || pending} onClick={() => void confirm()}>
          {t(confirmKey(step))}
        </Button>
      </div>
    </section>
  );
}
