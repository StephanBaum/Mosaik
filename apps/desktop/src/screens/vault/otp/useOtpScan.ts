/**
 * The state of the one-time code setup dialog: which step it is in, and the calls that
 * move it (`otp_scan`, `otp_parse`, `otp_pending_discard`). The decoded secret never
 * reaches this code from a scan; it holds only the preview and the pending ID.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { useToast } from "../../../components/ToastProvider.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type {
  IpcError,
  OtpParseRequest,
  OtpPending,
  OtpScanSource,
} from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import type { OtpStart } from "./OtpSection.tsx";

export type OtpStep =
  | { kind: "working"; source: OtpScanSource }
  | { kind: "manual" }
  /** The sources again, optionally with what went wrong. */
  | { kind: "choose"; error: IpcError | null }
  | { kind: "confirm"; pending: OtpPending; clipboardCleared: boolean };

export interface OtpScanApi {
  step: OtpStep;
  /** Whether a scan is running. */
  busy: boolean;
  scan: (source: OtpScanSource) => Promise<void>;
  /** Parses typed text; resolves with the error, or `null` after moving to the confirmation. */
  parse: (request: OtpParseRequest) => Promise<IpcError | null>;
  /** Back to the sources (forgets the pending setup). */
  choose: () => void;
  manual: () => void;
  /** Forgets the pending setup, if the editor did not take it. */
  discard: () => void;
  /** The editor took the pending setup: closing must not discard it. */
  handOver: () => void;
}

function firstStep(start: OtpStart): OtpStep {
  return start.kind === "scan" ? { kind: "working", source: start.source } : { kind: "manual" };
}

/** The one pending setup this dialog holds, and whether the dialog is still on screen. */
function usePendingSlot() {
  const ipc = useIpc();
  const pendingId = useRef<string | null>(null);
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const discardId = useCallback(
    (id: string | null) => {
      if (id !== null) {
        void ipc.call("otp_pending_discard", { pendingId: id });
      }
    },
    [ipc],
  );

  const discard = useCallback(() => {
    const id = pendingId.current;
    pendingId.current = null;
    discardId(id);
  }, [discardId]);

  const remember = useCallback((id: string) => {
    pendingId.current = id;
  }, []);

  const handOver = useCallback(() => {
    pendingId.current = null;
  }, []);

  const isMounted = useCallback(() => mounted.current, []);

  return { remember, handOver, isMounted, discardId, discard };
}

export function useOtpScan(start: OtpStart, onCancelled: () => void): OtpScanApi {
  const ipc = useIpc();
  const toast = useToast();
  const { t } = useI18n();
  const [step, setStep] = useState<OtpStep>(() => firstStep(start));
  const [busy, setBusy] = useState(false);
  const running = useRef(false);
  const { remember, handOver, isMounted, discardId, discard } = usePendingSlot();

  const accept = useCallback(
    (pending: OtpPending, clipboardCleared: boolean) => {
      if (!isMounted()) {
        discardId(pending.pendingId);
        return;
      }
      remember(pending.pendingId);
      setStep({ kind: "confirm", pending, clipboardCleared });
      if (clipboardCleared) {
        toast(t("otp.toast.clipboardCleared"), { tone: "success" });
      }
    },
    [remember, isMounted, discardId, toast, t],
  );

  const scan = useCallback(
    async (source: OtpScanSource) => {
      if (running.current) {
        return;
      }
      running.current = true;
      setBusy(true);
      discard();
      setStep({ kind: "working", source });
      const result = await ipc.call("otp_scan", { source });
      running.current = false;
      if (!isMounted()) {
        if (result.ok && result.value.kind === "found") {
          discardId(result.value.pending.pendingId);
        }
        return;
      }
      setBusy(false);
      if (!result.ok) {
        setStep({ kind: "choose", error: result.error });
      } else if (result.value.kind === "cancelled") {
        onCancelled();
      } else {
        accept(result.value.pending, result.value.clipboardCleared);
      }
    },
    [ipc, isMounted, discard, discardId, accept, onCancelled],
  );

  const parse = useCallback(
    async (request: OtpParseRequest): Promise<IpcError | null> => {
      discard();
      const result = await ipc.call("otp_parse", request);
      if (!result.ok) {
        return result.error;
      }
      accept(result.value, false);
      return null;
    },
    [ipc, discard, accept],
  );

  const choose = useCallback(() => {
    discard();
    setStep({ kind: "choose", error: null });
  }, [discard]);

  const manual = useCallback(() => {
    discard();
    setStep({ kind: "manual" });
  }, [discard]);

  return { step, busy, scan, parse, choose, manual, discard, handOver };
}
