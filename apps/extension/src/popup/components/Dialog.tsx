/**
 * A modal dialog on the native `<dialog>`: focus is trapped inside, Esc closes it (the top
 * layer first), and focus returns to where it was. Used for the frame confirmation and for
 * disconnecting. The cancel button has initial focus, so Enter never confirms by accident.
 */
import { useEffect, useId, useRef, type ReactNode } from "react";
import { useI18n } from "../i18n.tsx";
import { Button } from "./Button.tsx";

export function ConfirmDialog({
  title,
  children,
  confirmLabel,
  onConfirm,
  onCancel,
}: {
  title: string;
  children: ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const ref = useRef<HTMLDialogElement | null>(null);
  const titleId = useId();

  useEffect(() => {
    const dialog = ref.current;
    if (dialog === null) {
      return undefined;
    }
    const before = document.activeElement;
    dialog.showModal();
    return () => {
      dialog.close();
      if (before instanceof HTMLElement && before.isConnected) {
        before.focus();
      }
    };
  }, []);

  return (
    <dialog
      ref={ref}
      className="dialog"
      aria-labelledby={titleId}
      onCancel={(event) => {
        event.preventDefault();
        onCancel();
      }}
    >
      <h2 id={titleId} className="dialog__title">
        {title}
      </h2>
      <div className="dialog__body">{children}</div>
      <div className="dialog__actions">
        <Button autoFocus onClick={onCancel}>
          {t("ext.action.cancel")}
        </Button>
        <Button variant="primary" onClick={onConfirm}>
          {confirmLabel}
        </Button>
      </div>
    </dialog>
  );
}
