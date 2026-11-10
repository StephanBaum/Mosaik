/**
 * "Stop this change?": asked when a wizard that already prepared a job (the new Emergency Kit is
 * saved) is closed. Closing discards the job and the kit stops working, so Esc and Cancel ask
 * first, with the safe choice focused.
 */
import { Button } from "../../../components/Button.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";

export function StopDialog({ onStop, onKeep }: { onStop: () => void; onKeep: () => void }) {
  const { t } = useI18n();
  return (
    <Dialog title={t("rotation.stop.title")} onClose={onKeep}>
      <div className="stack">
        <p className="body">{t("rotation.stop.body")}</p>
        <div className="dialog__actions">
          <Button data-autofocus="" onClick={onKeep}>
            {t("rotation.stop.keep")}
          </Button>
          <Button variant="dangerSolid" onClick={onStop}>
            {t("rotation.stop.confirm")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
