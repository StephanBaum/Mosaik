/**
 * "Never offered for saving" (M4-R10): the sites the user chose "Never for this site" for, as
 * exact origins in plain text, each with a Remove button. The list lives in the background; only
 * this popup page can change it. The dialog has its own live region: the rest of the popup is
 * inert while a modal dialog is open.
 */
import { useEffect, useId, useRef, useState } from "react";
import { errorCodeOf } from "../../shared/errors.ts";
import { unicodeHostOf } from "../../shared/origin.ts";
import { useI18n } from "../i18n.tsx";
import { useServices } from "../usePopup.ts";
import { Button } from "./Button.tsx";

/** An origin for display: a punycode host is shown in Unicode (the ASCII origin goes in the title). */
function displayOrigin(origin: string): string {
  try {
    const url = new URL(origin);
    const unicode = unicodeHostOf(url.hostname);
    return unicode === null ? origin : `${url.protocol}//${unicode}${url.port === "" ? "" : `:${url.port}`}`;
  } catch {
    return origin;
  }
}

/** The list, the removal and the sentence the dialog's live region says. */
function useNeverSaveList() {
  const { t, errorText } = useI18n();
  const { api } = useServices();
  const [origins, setOrigins] = useState<string[] | null>(null);
  const [said, setSaid] = useState("");
  // The list could not be read: an error, never "No sites yet".
  const [failed, setFailed] = useState("");
  const closeRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    let current = true;
    api.request({ op: "neverSaveList" }).then(
      (result) => {
        if (current) {
          setOrigins(result.origins);
        }
      },
      (error: unknown) => {
        if (current) {
          setFailed(errorText(errorCodeOf(error)));
        }
      },
    );
    return () => {
      current = false;
    };
  }, [api, errorText]);

  const remove = (origin: string): void => {
    void api.request({ op: "neverSaveRemove", origin }).then(
      (result) => {
        setOrigins(result.origins);
        setSaid(t("ext.neverSave.removed", { origin: displayOrigin(origin) }));
        // The removed row's button is gone: keep focus inside the dialog.
        closeRef.current?.focus();
      },
      (error: unknown) => {
        setSaid(errorText(errorCodeOf(error)));
      },
    );
  };
  return { origins, said, failed, remove, closeRef };
}

export function NeverSaveDialog({ onClose }: { onClose: () => void }) {
  const { t } = useI18n();
  const ref = useRef<HTMLDialogElement | null>(null);
  const titleId = useId();
  const { origins, said, failed, remove, closeRef } = useNeverSaveList();

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
        onClose();
      }}
    >
      <h2 id={titleId} className="dialog__title">
        {t("ext.neverSave.title")}
      </h2>
      <div className="dialog__body">
        {failed !== "" && <p role="alert">{failed}</p>}
        {origins !== null && origins.length > 0 && <p>{t("ext.neverSave.intro")}</p>}
        {origins !== null && origins.length === 0 && <p className="muted">{t("ext.neverSave.empty")}</p>}
        {origins !== null && origins.length > 0 && (
          <ul className="never-list">
            {origins.map((origin) => (
              <li key={origin} className="never-list__row">
                <span className="never-list__origin" title={origin}>
                  {displayOrigin(origin)}
                </span>
                <Button
                  onClick={() => {
                    remove(origin);
                  }}
                  aria-label={t("ext.neverSave.removeLabel", { origin: displayOrigin(origin) })}
                >
                  {t("ext.neverSave.remove")}
                </Button>
              </li>
            ))}
          </ul>
        )}
        <p className="caption" role="status">
          {said}
        </p>
      </div>
      <div className="dialog__actions">
        <Button ref={closeRef} autoFocus variant="primary" onClick={onClose}>
          {t("ext.neverSave.close")}
        </Button>
      </div>
    </dialog>
  );
}
