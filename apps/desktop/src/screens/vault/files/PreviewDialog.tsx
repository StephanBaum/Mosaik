/**
 * The image preview (ADR-0034): Rust decodes the file and sends downscaled RGBA pixels, which
 * are painted into a canvas. The dialog is treated like a revealed secret: it closes on lock
 * and when the window loses focus (`useSecrets().register`), the canvas is cleared and shrunk
 * to nothing when it goes, and the answer's buffer is zeroed right after painting. An answer
 * that arrives after the dialog closed is zeroed and dropped.
 *
 * The frame carries `data-secret` without `data-mask`: the reveal snapshot paints its box and
 * never visits what is in it (the dialog itself is skipped as well).
 */
import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { useSecrets } from "../../../app/SecretsContext.tsx";
import { Button } from "../../../components/Button.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { useLatest } from "../../../hooks/useLatest.ts";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { AttachmentEntry, IpcError, ItemRequest } from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import { paintPreview, parsePreview, wipeBuffer, wipeCanvas } from "./preview.ts";

/** Rust decodes one preview at a time; a request that meets a running one waits and asks again. */
const BUSY_RETRIES = 8;
const BUSY_WAIT_MS = 250;

const INTERNAL: IpcError = { code: "internal" };

interface Load {
  loading: boolean;
  error: IpcError | null;
}

/** Asks for the pixels and paints them, unless the dialog was closed in the meantime. */
function usePreviewLoad(
  item: ItemRequest,
  attachmentId: string,
  canvasRef: RefObject<HTMLCanvasElement | null>,
  generationRef: RefObject<number>,
): Load {
  const ipc = useIpc();
  const [load, setLoad] = useState<Load>({ loading: true, error: null });
  const { vaultId, itemId } = item;
  useEffect(() => {
    const canvas = canvasRef.current;
    generationRef.current += 1;
    const asked = generationRef.current;
    const current = () => generationRef.current === asked;
    async function run() {
      for (let attempt = 0; ; attempt += 1) {
        const result = await ipc.call("attachment_preview", { vaultId, itemId, attachmentId });
        if (!result.ok) {
          if (current() && result.error.code === "busy" && attempt < BUSY_RETRIES) {
            await new Promise((resolve) => setTimeout(resolve, BUSY_WAIT_MS));
            if (current()) {
              continue;
            }
          } else if (current()) {
            setLoad({ loading: false, error: result.error });
          }
          return;
        }
        const buffer = result.value;
        try {
          if (current() && canvas !== null) {
            paintPreview(canvas, parsePreview(buffer));
            setLoad({ loading: false, error: null });
          }
        } catch {
          wipeCanvas(canvas);
          setLoad({ loading: false, error: INTERNAL });
        } finally {
          wipeBuffer(buffer);
        }
        return;
      }
    }
    void run();
    return () => {
      generationRef.current += 1;
      wipeCanvas(canvas);
    };
  }, [ipc, vaultId, itemId, attachmentId, canvasRef, generationRef]);
  return load;
}

export function PreviewDialog({
  item,
  attachment,
  onClose,
  onSaveAs,
}: {
  item: ItemRequest;
  attachment: AttachmentEntry;
  onClose: () => void;
  onSaveAs: () => void;
}) {
  const { t } = useI18n();
  const secrets = useSecrets();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const generationRef = useRef(0);
  const closeNow = useLatest(onClose);
  const { loading, error } = usePreviewLoad(item, attachment.attachmentId, canvasRef, generationRef);

  // Esc, the Close button, lock and blur all end up here.
  const close = useCallback(() => {
    generationRef.current += 1;
    wipeCanvas(canvasRef.current);
    closeNow.current();
  }, [closeNow]);
  useEffect(() => secrets.register(close), [secrets, close]);

  return (
    <Dialog title={attachment.fileName} size="wide" onClose={close}>
      {loading && (
        <p role="status" className="files__note preview__loading">
          {t("files.previewLoading")}
        </p>
      )}
      <div
        data-secret=""
        className="preview__frame"
        aria-busy={loading}
        hidden={loading || error !== null}
      >
        <canvas ref={canvasRef} role="img" aria-label={t("files.previewAlt", { name: attachment.fileName })} />
      </div>
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onSaveAs}>{t("files.saveAsShort")}</Button>
        <Button data-autofocus="" onClick={close}>
          {t("files.close")}
        </Button>
      </div>
    </Dialog>
  );
}
