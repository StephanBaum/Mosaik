/**
 * A modal dialog on the native `<dialog>` element: focus is trapped and Esc closes the
 * top-most dialog without extra code. Put `autoFocus` on the control that should start
 * with focus.
 */
import { useEffect, useRef, type ReactNode } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { IconButton } from "./IconButton.tsx";

export function Dialog({
  title,
  onClose,
  children,
  label,
  size = "md",
  hideTitle = false,
  closeButton = false,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  /** Names the dialog for screen readers when it differs from the visible title. */
  label?: string;
  size?: "md" | "wide" | "palette";
  /** Keeps the title for screen readers only. */
  hideTitle?: boolean;
  /** Adds a close button at the top right. */
  closeButton?: boolean;
}) {
  const { t } = useI18n();
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = ref.current;
    if (dialog !== null && !dialog.open) {
      dialog.showModal();
    }
  }, []);

  return (
    <dialog
      ref={ref}
      className={`dialog dialog--${size}`}
      aria-label={label ?? title}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <div className={hideTitle && !closeButton ? "visually-hidden" : "dialog__header"}>
        <h2 className={hideTitle ? "visually-hidden" : "dialog__title"}>{title}</h2>
        {closeButton && (
          <IconButton icon="x" label={t("action.close")} onClick={onClose} className="dialog__close" />
        )}
      </div>
      {children}
    </dialog>
  );
}
