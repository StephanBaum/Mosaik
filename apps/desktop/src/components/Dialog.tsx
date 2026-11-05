/**
 * A modal dialog on the native `<dialog>` element: focus is trapped and Esc closes the
 * top-most dialog without extra code. Put `data-autofocus` on the control that should start
 * with focus (React's `autoFocus` runs before `showModal()` and is ignored by a closed dialog).
 * Tab and Shift+Tab wrap inside the dialog, so leaving it by keyboard never moves focus to the
 * page behind (the preview closes when the window loses focus).
 */
import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { IconButton } from "./IconButton.tsx";

const FOCUSABLE =
  'a[href], button, input, select, textarea, [tabindex]:not([tabindex="-1"])';

/** The controls Tab can reach inside `dialog`, in order. */
function tabStops(dialog: HTMLElement): HTMLElement[] {
  return Array.from(dialog.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (element) =>
      !element.hasAttribute("disabled") &&
      element.getAttribute("tabindex") !== "-1" &&
      element.closest("[hidden], [inert]") === null,
  );
}

/** Wraps Tab from the last control to the first and Shift+Tab the other way. */
function trapTab(event: KeyboardEvent<HTMLDialogElement>): void {
  if (event.key !== "Tab") {
    return;
  }
  const stops = tabStops(event.currentTarget);
  const first = stops[0];
  const last = stops[stops.length - 1];
  if (first === undefined || last === undefined) {
    event.preventDefault();
    return;
  }
  const active = document.activeElement;
  if (event.shiftKey && (active === first || active === event.currentTarget)) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
  }
}

export function Dialog({
  title,
  onClose,
  children,
  label,
  size = "md",
  hideTitle = false,
  closeButton = false,
  icon,
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
  /** A decorative tile above the title. */
  icon?: ReactNode;
}) {
  const { t } = useI18n();
  const ref = useRef<HTMLDialogElement>(null);

  // Read while rendering, before the dialog or its `autoFocus` control takes focus.
  const [opener] = useState(() => document.activeElement);

  useEffect(() => {
    const dialog = ref.current;
    if (dialog !== null && !dialog.open) {
      dialog.showModal();
      dialog.querySelector<HTMLElement>("[data-autofocus]")?.focus();
    }
    // Whatever had focus when the dialog opened gets it back when the dialog goes away.
    return () => {
      if (opener instanceof HTMLElement && opener.isConnected && opener !== document.body) {
        opener.focus();
      }
    };
  }, [opener]);

  return (
    <dialog
      ref={ref}
      className={`dialog dialog--${size}`}
      aria-label={label ?? title}
      onKeyDown={trapTab}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      {icon !== undefined && (
        <div className="dialog__icon" aria-hidden="true">
          {icon}
        </div>
      )}
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
