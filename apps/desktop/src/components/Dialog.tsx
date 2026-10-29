/**
 * A modal dialog on the native `<dialog>` element: focus is trapped and Esc closes the
 * top-most dialog without extra code. Put `autoFocus` on the control that should start
 * with focus.
 */
import { useEffect, useRef, type ReactNode } from "react";

export function Dialog({
  title,
  onClose,
  children,
  label,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  /** Names the dialog for screen readers when it differs from the visible title. */
  label?: string;
}) {
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
      className="dialog"
      aria-label={label ?? title}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <h2 className="dialog__title">{title}</h2>
      {children}
    </dialog>
  );
}
