/** A short notice next to the field the user was in, with a Close button (ADR-0164). */
import { useState } from "react";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError, PickerNotice, PickerState } from "../ipc/contract/index.ts";
import type { PickerIpc } from "./ipc.ts";

const NOTICE_KEYS = {
  elevated: "programFill.notice.elevated",
  noField: "programFill.notice.noField",
  aborted: "programFill.notice.aborted",
  abortedCleared: "programFill.notice.abortedCleared",
  modifiersHeld: "programFill.notice.modifiersHeld",
  notResponding: "programFill.notice.notResponding",
} as const satisfies Record<PickerNotice, string>;

export function NoticeView({ state, ipc }: { state: PickerState; ipc: PickerIpc }) {
  const { t } = useI18n();
  const [error, setError] = useState<IpcError | null>(null);
  const notice = state.notice;

  const close = async (): Promise<void> => {
    const result = await ipc.call("picker_dismiss", { generation: state.generation });
    setError(result.ok ? null : result.error);
  };

  return (
    <section className="picker picker--notice" role="status">
      <p className="picker__text">{notice === null ? "" : t(NOTICE_KEYS[notice])}</p>
      <ErrorAlert error={error} />
      <div className="picker__actions">
        <Button size="sm" onClick={() => void close()}>
          {t("programFill.notice.close")}
        </Button>
      </div>
    </section>
  );
}
