/** Toasts in one polite live region (UX_GUIDELINES §7). */
import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";

interface Toast {
  id: number;
  message: string;
}

const TOAST_MS = 6000;

const ToastContext = createContext<((message: string) => void) | null>(null);

export function ToastProvider({ children }: { children: ReactNode }) {
  const { t } = useI18n();
  const [toasts, setToasts] = useState<Toast[]>([]);
  const next = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);
  const show = useCallback(
    (message: string) => {
      next.current += 1;
      const id = next.current;
      setToasts((current) => [...current, { id, message }]);
      setTimeout(() => {
        dismiss(id);
      }, TOAST_MS);
    },
    [dismiss],
  );
  const value = useMemo(() => show, [show]);

  return (
    <ToastContext value={value}>
      {children}
      <div className="toasts" role="status" aria-live="polite" aria-label={t("toast.region")}>
        {toasts.map((toast) => (
          <p key={toast.id} className="toast">
            <span>{toast.message}</span>
            <button
              type="button"
              className="toast__close"
              aria-label={t("toast.dismiss")}
              onClick={() => {
                dismiss(toast.id);
              }}
            >
              {t("action.close")}
            </button>
          </p>
        ))}
      </div>
    </ToastContext>
  );
}

/** Shows a message in the toast region. */
export function useToast(): (message: string) => void {
  const context = useContext(ToastContext);
  if (context === null) {
    throw new Error("useToast needs a ToastProvider");
  }
  return context;
}
