/** Copies for the guided change: the new password through Rust, the change-page link as plain text. */
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { RotationAttemptRequest } from "../../ipc/contract/index.ts";

export interface AttemptCopies {
  /** Copies the new password of an open attempt (Rust writes the clipboard and clears it). */
  password: (request: RotationAttemptRequest) => Promise<void>;
  /** Copies the change-page link (not a secret), through the webview's clipboard. */
  link: (url: string) => Promise<void>;
}

export function useCopyAttempt(): AttemptCopies {
  const { t, errorMessage } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  return {
    password: async (request) => {
      const result = await ipc.call("rotation_copy", request);
      if (!result.ok) {
        toast(errorMessage(result.error));
      } else if (result.value.clearAfterSeconds === null) {
        toast(t("toast.copiedKept"));
      } else {
        toast(t("toast.copied", { seconds: result.value.clearAfterSeconds }), {
          tone: "success",
          action: {
            label: t("clipboard.clearNow"),
            run: () => {
              void ipc.call("clear_clipboard", null);
            },
          },
        });
      }
    },
    link: async (url) => {
      try {
        await navigator.clipboard.writeText(url);
        toast(t("credentialRotation.steps.site.linkCopied"), { tone: "success" });
      } catch {
        toast(errorMessage({ code: "clipboardUnavailable" }));
      }
    },
  };
}
