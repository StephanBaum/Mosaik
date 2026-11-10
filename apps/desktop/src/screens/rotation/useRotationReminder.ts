/**
 * After an unlock, with the reminder setting on: one message in the app, "n passwords to change",
 * with "Show" (M7b). It reads the local queue; no request leaves the device for it.
 */
import { useEffect, useRef } from "react";
import { useSettings } from "../../app/SettingsProvider.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { dueCount } from "./useRotationQueue.ts";

export function useRotationReminder(onShow: () => void): void {
  const ipc = useIpc();
  const toast = useToast();
  const { t } = useI18n();
  const { settings } = useSettings();
  const asked = useRef(false);
  const enabled = settings?.rotationReminders === true;
  useEffect(() => {
    if (!enabled || asked.current) {
      return;
    }
    asked.current = true;
    void ipc.call("rotation_queue", null).then((result) => {
      const count = result.ok ? dueCount(result.value) : 0;
      if (count > 0) {
        toast(t("credentialRotation.reminder.toast", { count }), {
          action: { label: t("credentialRotation.reminder.show"), run: onShow },
        });
      }
    });
  }, [enabled, ipc, toast, t, onShow]);
}
