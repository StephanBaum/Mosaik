/** Copies a field through Rust (history exclusion, auto-clear) and says so in a toast. */
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { CopySource } from "../ipc/contract/index.ts";
import { Button } from "./Button.tsx";
import { useToast } from "./ToastProvider.tsx";

/** Copies `source` through Rust and reports the outcome in a toast. */
export function useCopy(): (source: CopySource) => Promise<void> {
  const { t, errorMessage } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  return async (source) => {
    const result = await ipc.call("copy_to_clipboard", { source });
    if (!result.ok) {
      toast(errorMessage(result.error));
    } else if (result.value.clearAfterSeconds === null) {
      toast(t("toast.copiedKept"));
    } else {
      toast(t("toast.copied", { seconds: result.value.clearAfterSeconds }));
    }
  };
}

export function CopyButton({ source, field }: { source: CopySource; field: string }) {
  const { t } = useI18n();
  const copy = useCopy();

  return (
    <Button
      variant="ghost"
      aria-label={t("item.copy", { field })}
      onClick={() => {
        void copy(source);
      }}
    >
      {t("action.copy")}
    </Button>
  );
}
