/** A failed command, as a sentence that says what happened and what to do next. */
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError } from "../ipc/contract/index.ts";

/** `message` replaces the generic sentence of `error` where a screen knows better what to say. */
export function ErrorAlert({ error, message }: { error: IpcError | null; message?: string | undefined }) {
  const { errorMessage } = useI18n();
  // The live region stays mounted so screen readers announce new errors.
  return (
    <p className="alert" role="alert">
      {error === null ? "" : (message ?? errorMessage(error))}
    </p>
  );
}
