/**
 * "Breach checks" on the security page (D13): the opt-in check against Have I Been Pwned. The
 * mode (off, manual, weekly), "Check now", the progress and the last result. Off is the
 * default and nothing leaves the device while it is off; the first "Check now" and every switch
 * from off asks for the consent first.
 *
 * Live region: one polite status that changes when a scan starts and when it ends. The step
 * counter is for the eyes only, so a screen reader is not told about every prefix.
 */
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Select } from "../../components/Select.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { breachModes, type BreachMode, type BreachStatus } from "../../ipc/contract/index.ts";
import { BreachConsentDialog } from "./BreachConsentDialog.tsx";
import { useBreachScan, type BreachScan, type ScanSummary } from "./useBreachScan.ts";

type Translate = ReturnType<typeof useI18n>["t"];

/** What the card says: the result line, and the error (a warning of its own) if the scan stopped early. */
interface Report {
  text: string;
  error: string | null;
}

/** The result of the scan that just ended, or of the last scan the status remembers. */
function resultReport(t: Translate, status: BreachStatus, finished: ScanSummary | null, found: number): Report {
  if (finished !== null) {
    const error = finished.error === null ? null : t(`breach.error.${finished.error}`);
    if (finished.checked > 0 || finished.error === null) {
      return {
        error,
        text:
          finished.checked === 0
            ? t("breach.result.upToDate")
            : t("breach.result.finished", { checked: finished.checked, found: finished.found }),
      };
    }
    return { error, text: "" };
  }
  return {
    error: status.lastError === null ? null : t(`breach.error.${status.lastError}`),
    text:
      status.lastFinishedAt === null
        ? t("breach.result.never")
        : t("breach.result.last", { date: status.lastFinishedAt, found }),
  };
}

function report(t: Translate, scan: BreachScan, status: BreachStatus, found: number): Report {
  if (scan.running) {
    return { text: t("breach.progress.starting"), error: null };
  }
  if (status.mode === "off" && scan.finished === null) {
    return { text: t("breach.card.off"), error: null };
  }
  return resultReport(t, status, scan.finished, found);
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
  const shown = report(t, scan, status, found);
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
        {shown.text}
      </p>
      {shown.error !== null && <Callout tone="warning">{shown.error}</Callout>}
      {scan.running && scan.progress !== null && (
        <p className="caption tabular" aria-hidden="true">
          {t("breach.progress.step", scan.progress)}
        </p>
      )}
      <ErrorAlert error={scan.error} />
      <div className="actions">
        {/* One button whose label and action change, so focus stays on it. */}
        <HoloButton onClick={scan.running ? scan.stop : scan.checkNow}>
          {t(scan.running ? "breach.card.stop" : "breach.card.checkNow")}
        </HoloButton>
      </div>
      {scan.consenting !== null && <BreachConsentDialog onAgree={scan.agree} onDecline={scan.decline} />}
    </section>
  );
}
