/** Toasts in one polite live region (UX_GUIDELINES §7). */
import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { Button } from "./Button.tsx";
import { Icon } from "./icons.tsx";
import { IconButton } from "./IconButton.tsx";

export interface ToastOptions {
  tone?: "success" | "info";
  action?: { label: string; run: () => void };
}

interface Toast extends ToastOptions {
  id: number;
  message: string;
}

const TOAST_MS = 6000;

type Show = (message: string, options?: ToastOptions) => void;

const ToastContext = createContext<Show | null>(null);

export function ToastProvider({ children }: { children: ReactNode }) {
  const { t } = useI18n();
  const [toasts, setToasts] = useState<Toast[]>([]);
  const next = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);
  const show = useCallback<Show>(
    (message, options = {}) => {
      next.current += 1;
      const id = next.current;
      // The same message again (a run of "Settings saved.") replaces the one on screen.
      setToasts((current) => [
        ...current.filter((toast) => toast.message !== message || toast.action !== undefined),
        { id, message, ...options },
      ]);
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
          <div key={toast.id} className="toast">
            {toast.tone === "success" && (
              <span className="toast__badge">
                <Icon name="check" size="sm" />
              </span>
            )}
            <span className="toast__message">{toast.message}</span>
            {toast.action !== undefined && (
              <Button
                size="sm"
                onClick={() => {
                  toast.action?.run();
                  dismiss(toast.id);
                }}
              >
                {toast.action.label}
              </Button>
            )}
            <IconButton
              icon="x"
              size="sm"
              label={t("toast.dismiss")}
              onClick={() => {
                dismiss(toast.id);
              }}
            />
          </div>
        ))}
      </div>
    </ToastContext>
  );
}

/** Shows a message in the toast region. */
export function useToast(): Show {
  const context = useContext(ToastContext);
  if (context === null) {
    throw new Error("useToast needs a ToastProvider");
  }
  return context;
}
