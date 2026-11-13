/**
 * "Set up a phone" (M10, ADR-0096): a QR code that holds the server, the email and the Secret Key
 * of this hosted account, so a phone can join without typing them. It asks for the master
 * password like every view of the Secret Key. Rust builds and renders the code; this file only
 * draws the module matrix on a canvas, and hides it after 60 seconds, when the window loses
 * focus or is hidden, and on lock (the screen unmounts then). A folder account gets a sentence
 * instead: the phone apps support Mosaic sync for now.
 */
import { useEffect, useRef, useState } from "react";
import { useLatest } from "../../hooks/useLatest.ts";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ReauthDialog } from "../../components/ReauthDialog.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { SetupCodeShown, SyncConfigView } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";

/** Canvas pixels per module, and modules of quiet zone around the code. */
const SCALE = 8;
const QUIET = 4;
/** The longest the code stays up, whatever the clock in the answer says. */
const MAX_VISIBLE_MS = 60_000;

type Away = "time" | "away";

/** Draws the code in the print colors (dark ink on white), which a camera reads in both themes. */
function draw(canvas: HTMLCanvasElement, shown: SetupCodeShown): void {
  const context = canvas.getContext("2d");
  if (context === null) {
    return;
  }
  const style = getComputedStyle(canvas);
  const ink = style.getPropertyValue("--print-text").trim();
  const paper = style.getPropertyValue("--print-bg").trim();
  const side = (shown.size + 2 * QUIET) * SCALE;
  context.fillStyle = paper === "" ? "Canvas" : paper;
  context.fillRect(0, 0, side, side);
  context.fillStyle = ink === "" ? "CanvasText" : ink;
  shown.modules.forEach((row, y) => {
    row.forEach((dark, x) => {
      if (dark) {
        context.fillRect((x + QUIET) * SCALE, (y + QUIET) * SCALE, SCALE, SCALE);
      }
    });
  });
}

function CodeDialog({
  shown,
  onHide,
  onClose,
}: {
  shown: SetupCodeShown;
  /** The code must go: the time ran out, or the window was left. */
  onHide: (why: Away) => void;
  /** The user closed the dialog. */
  onClose: () => void;
}) {
  const { t } = useI18n();
  const canvas = useRef<HTMLCanvasElement>(null);
  // The timer belongs to this code: a new `onHide` from a re-render must not restart it.
  const hide = useLatest(onHide);
  useEffect(() => {
    if (canvas.current !== null) {
      draw(canvas.current, shown);
    }
  }, [shown]);
  useEffect(() => {
    const left = Math.max(0, Math.min(MAX_VISIBLE_MS, shown.expiresAt - Date.now()));
    const timer = window.setTimeout(() => {
      hide.current("time");
    }, left);
    const away = () => {
      hide.current("away");
    };
    const hidden = () => {
      if (document.hidden) {
        away();
      }
    };
    window.addEventListener("blur", away);
    document.addEventListener("visibilitychange", hidden);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener("blur", away);
      document.removeEventListener("visibilitychange", hidden);
    };
  }, [shown, hide]);
  const side = (shown.size + 2 * QUIET) * SCALE;
  return (
    <Dialog
      title={t("settings.sync.phone.dialogTitle")}
      closeButton
      onClose={onClose}
    >
      <div className="stack">
        <p className="body muted">{t("settings.sync.phone.steps")}</p>
        <canvas
          ref={canvas}
          className="setup-code"
          width={side}
          height={side}
          role="img"
          aria-label={t("settings.sync.phone.canvas")}
          data-secret=""
        />
        <p className="caption">{t("settings.sync.phone.hint")}</p>
      </div>
    </Dialog>
  );
}

export function SetupPhone({ config }: { config: SyncConfigView }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [ask, setAsk] = useState(false);
  const [shown, setShown] = useState<SetupCodeShown | null>(null);
  const [away, setAway] = useState<Away | null>(null);

  if (config.kind !== "hosted") {
    return <p className="field__hint">{t("settings.sync.phone.folder")}</p>;
  }
  return (
    <div className="stack">
      <h3 className="subtitle">{t("settings.sync.phone.title")}</h3>
      <p className="caption">{t("settings.sync.phone.body")}</p>
      {away !== null && (
        <p className="caption" role="status">
          {t(away === "time" ? "settings.sync.phone.hidden.time" : "settings.sync.phone.hidden.away")}
        </p>
      )}
      <div className="actions">
        <Button
          onClick={() => {
            setAsk(true);
          }}
        >
          {away === null ? t("settings.sync.phone.button") : t("settings.sync.phone.again")}
        </Button>
      </div>
      {ask && (
        <ReauthDialog
          body="settings.sync.phone.reauth"
          onSubmit={async (password) => {
            const result = await ipc.call("setup_code_show", { password });
            if (!result.ok) {
              return result.error;
            }
            setAway(null);
            setShown(result.value);
            return null;
          }}
          onClose={() => {
            setAsk(false);
          }}
        />
      )}
      {shown !== null && (
        <CodeDialog
          shown={shown}
          onHide={(why) => {
            setShown(null);
            setAway(why);
          }}
          onClose={() => {
            setShown(null);
          }}
        />
      )}
    </div>
  );
}
