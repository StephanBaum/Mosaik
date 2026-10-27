/** A failed command, as a sentence that says what happened and what to do next. */
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError } from "../ipc/contract/index.ts";

export function ErrorAlert({ error }: { error: IpcError | null }) {
  const { errorMessage } = useI18n();
  // The live region stays mounted so screen readers announce new errors.
  return (
    <p className="alert" role="alert">
      {error === null ? "" : errorMessage(error)}
    </p>
  );
}
