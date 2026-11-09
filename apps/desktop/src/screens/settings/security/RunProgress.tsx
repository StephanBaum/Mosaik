/**
 * The phases of a running key change with a count for the attachments. Only the name of the
 * current phase is in the live region (it changes a handful of times); the counts change every
 * second or two and are plain text, so a screen reader is not read a timer.
 */
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { RekeyBackend, RotationJobView, RotationPhaseName } from "../../../ipc/contract/index.ts";

/** The phases a backend goes through, in the order the user sees them. */
function phasesOf(backend: RekeyBackend): RotationPhaseName[] {
  return backend === "folder"
    ? ["localCommitted", "folderWritten", "attachments", "uploading"]
    : ["backendStarted", "backendAccepted", "localCommitted", "attachments", "uploading"];
}

export function RunProgress({
  job,
  backend,
}: {
  /** `null` before the first answer: everything is waiting. */
  job: RotationJobView | null;
  backend: RekeyBackend;
}) {
  const { t } = useI18n();
  const phases = phasesOf(backend);
  const index = job === null ? -1 : phases.indexOf(job.phase);
  const label = index < 0 ? t("rotation.phase.prepared") : t(`rotation.phase.${phases[index] ?? "prepared"}`);
  return (
    <div className="progress" role="group" aria-label={t("rotation.progress.label")}>
      <div className="progress__bar" aria-hidden="true">
        {phases.map((phase, position) => (
          <span key={phase} className={position <= index ? "progress__seg progress__seg--done" : "progress__seg"} />
        ))}
      </div>
      <p className="body" role="status" aria-live="polite">
        {label}
      </p>
      <ol className="stack">
        {phases.map((phase, position) => (
          <li
            key={phase}
            className={position === index ? "body" : "caption"}
            {...(position === index ? { "aria-current": "step" as const } : {})}
          >
            {t(`rotation.phase.${phase}`)}
            {phase === "attachments" && job !== null && job.phase === "attachments" && job.total > 0 && (
              <span> ({t("rotation.progress.count", { done: job.done, total: job.total })})</span>
            )}
          </li>
        ))}
      </ol>
    </div>
  );
}
