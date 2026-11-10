/**
 * "Breach checks" on the security page (D13): the opt-in check against Have I Been Pwned. The
 * mode (off, manual, weekly), "Check now", the progress and the last result. Off is the
 * default and nothing leaves the device while it is off; the first "Check now" and every switch
 * from off asks for the consent first.
 *
 * Live region: one polite status that changes when a scan starts and when it ends. The step
 * counter is for the eyes only, so a screen reader is not told about every prefix.
 */
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Select } from "../../components/Select.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { breachModes, type BreachMode, type BreachStatus } from "../../ipc/contract/index.ts";
import { BreachConsentDialog } from "./BreachConsentDialog.tsx";
import { useBreachScan, type BreachScan, type ScanSummary } from "./useBreachScan.ts";

type Translate = ReturnType<typeof useI18n>["t"];

/** The sentence for the end of a scan, or of the last scan the status remembers. */
function resultText(t: Translate, status: BreachStatus, finished: ScanSummary | null, found: number): string {
  if (finished !== null) {
    const parts: string[] = [];
    if (finished.error !== null) {
      parts.push(t(`breach.error.${finished.error}`));
    }
    if (finished.checked > 0 || finished.error === null) {
      parts.push(
        finished.checked === 0
          ? t("breach.result.upToDate")
          : t("breach.result.finished", { checked: finished.checked, found: finished.found }),
      );
    }
    return parts.join(" ");
  }
  const parts: string[] = [];
  if (status.lastError !== null) {
    parts.push(t(`breach.error.${status.lastError}`));
  }
  parts.push(
    status.lastFinishedAt === null
      ? t("breach.result.never")
      : t("breach.result.last", { date: status.lastFinishedAt, found }),
  );
  return parts.join(" ");
}

function statusText(t: Translate, scan: BreachScan, status: BreachStatus, found: number): string {
  if (scan.running) {
    return t("breach.progress.starting");
  }
  if (status.mode === "off" && scan.finished === null) {
    return t("breach.card.off");
  }
  return resultText(t, status, scan.finished, found);
}

export function BreachCheckCard({
  status,
  found,
  onChanged,
}: {
  status: BreachStatus;
  /** Passwords the report found in a breach (the status does not carry a count). */
  found: number;
  /** The mode or the findings changed: read the queue and the report again. */
  onChanged: () => void;
}) {
  const { t } = useI18n();
  const scan = useBreachScan(status, onChanged);
  const options = breachModes.map((mode) => ({ value: mode, label: t(`breach.mode.${mode}`) }));
  return (
    <section className="card breach" aria-labelledby="breach-title" data-breach-card="">
      <div className="breach__header">
        <h2 id="breach-title" className="subtitle">
          {t("breach.card.title")}
        </h2>
        <p className="caption">{t("breach.card.about")}</p>
      </div>
      <Select
        label={t("breach.mode.label")}
        value={status.mode}
        options={options}
        onChange={(event) => {
          scan.setMode(event.target.value as BreachMode);
        }}
      />
      <p role="status" aria-live="polite" className="breach__status">
        {statusText(t, scan, status, found)}
      </p>
      {scan.running && scan.progress !== null && (
        <p className="caption tabular" aria-hidden="true">
          {t("breach.progress.step", scan.progress)}
        </p>
      )}
      <ErrorAlert error={scan.error} />
      <div className="actions">
        {scan.running ? (
          <Button onClick={scan.stop}>{t("breach.card.stop")}</Button>
        ) : (
          <HoloButton onClick={scan.checkNow}>{t("breach.card.checkNow")}</HoloButton>
        )}
      </div>
      {scan.consenting !== null && <BreachConsentDialog onAgree={scan.agree} onDecline={scan.decline} />}
    </section>
  );
}
