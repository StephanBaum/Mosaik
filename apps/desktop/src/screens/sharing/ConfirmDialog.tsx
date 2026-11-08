/** A question before an action that can't be taken back or that tells someone else: "Cancel" has focus. */
import type { ReactNode } from "react";
import { Button, type ButtonVariant } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";

export function ConfirmDialog({
  title,
  body,
  confirmLabel,
  variant = "dangerSolid",
  onConfirm,
  onClose,
}: {
  title: string;
  body: ReactNode;
  confirmLabel: string;
  variant?: ButtonVariant;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <Dialog title={title} onClose={onClose}>
      <div className="stack">
        <p className="body muted">{body}</p>
        <div className="dialog__actions">
          <Button data-autofocus="" onClick={onClose}>
            {t("action.cancel")}
          </Button>
          <Button
            variant={variant}
            onClick={() => {
              onClose();
              onConfirm();
            }}
          >
            {confirmLabel}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
