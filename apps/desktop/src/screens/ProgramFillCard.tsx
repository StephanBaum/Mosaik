/**
 * "Fill logins in programs too": the card after an unlock on a Windows device where program fill
 * is available, off and not answered before (ADR-0164). "Turn on" switches it on and ends the
 * card; "Not now" only ends the card (it never comes back by itself; Settings has the switch).
 * Neither needs the master password.
 */
import { useState } from "react";
import { useSettings } from "../app/SettingsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { Callout } from "../components/Callout.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError } from "../ipc/contract/index.ts";
import { useProgramFillStatus } from "./ProgramFillSettings.tsx";

export function ProgramFillCard() {
  const { t } = useI18n();
  const { settings, update } = useSettings();
  const { status } = useProgramFillStatus(settings);
  const [error, setError] = useState<IpcError | null>(null);

  if (settings === null || status === null) {
    return null;
  }
  const show = status.available && !settings.programFill && !settings.programFillCardDismissed;
  const answer = async (turnOn: boolean): Promise<void> => {
    setError(await update({ ...settings, programFill: turnOn, programFillCardDismissed: true }, null));
  };
  // The live region stays mounted, so the card is announced when it appears.
  return (
    <div role="status">
      {show && (
        <Callout tone="info">
          <p className="body">
            <strong>{t("programFill.card.title")}</strong>
          </p>
          <p className="body">{t("programFill.settings.description")}</p>
          <ErrorAlert error={error} />
          <div className="actions">
            <Button
              onClick={() => {
                void answer(true);
              }}
            >
              {t("programFill.card.turnOn")}
            </Button>
            <Button
              variant="ghost"
              onClick={() => {
                void answer(false);
              }}
            >
              {t("programFill.card.notNow")}
            </Button>
          </div>
        </Callout>
      )}
    </div>
  );
}
