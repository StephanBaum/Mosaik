/**
 * The frame confirmation (docs/EXTENSION_SECURITY.md, "frames"). A fill into a same-origin
 * frame needs a click on "Fill in the frame". The click names the very frame (and document)
 * the popup showed when the dialog opened; the background refuses it if the target at fill
 * time is another one. A frame of another site is never offered. The change fill has its own
 * frame (`changeFrame`: where the change form is, which can differ from the sign-in form's).
 */
import { useState, type ReactNode } from "react";
import type { FrameRef, PopupState } from "../../shared/messages.ts";
import { useI18n } from "../i18n.tsx";
import { ConfirmDialog } from "./Dialog.tsx";

type Run = (confirm: FrameRef | null) => Promise<void>;

interface Pending {
  run: Run;
  target: FrameRef | null;
}

/** Which frame a fill is about: the sign-in form's, or the change form's. */
export type GateFor = "login" | "change";

export interface FrameGate {
  /** Runs `run` now, or after the user confirmed the frame. */
  gate: (run: Run, which?: GateFor) => void;
  /** The field in focus is in a frame of another site: nothing may be filled. */
  blocked: boolean;
  /** The same for the change fill, whose frame is the change form's. */
  blockedChange: boolean;
  dialog: ReactNode;
}

function frameOf(state: PopupState, which: GateFor): { frame: PopupState["frame"]; target: FrameRef | null } {
  if (which === "change" && state.changeFrame !== null) {
    return state.changeFrame;
  }
  return { frame: state.frame, target: state.frameTarget };
}

export function useFrameGate(state: PopupState): FrameGate {
  const { t } = useI18n();
  const [pending, setPending] = useState<Pending | null>(null);
  const dialog =
    pending === null ? null : (
      <ConfirmDialog
        title={t("ext.frameConfirm.title")}
        confirmLabel={t("ext.frameConfirm.confirm")}
        onCancel={() => {
          setPending(null);
        }}
        onConfirm={() => {
          setPending(null);
          void pending.run(pending.target);
        }}
      >
        <p>{t("ext.frameConfirm.body", { host: state.tab?.host ?? "" })}</p>
      </ConfirmDialog>
    );
  return {
    blocked: state.frame === "crossOriginFrame",
    blockedChange: frameOf(state, "change").frame === "crossOriginFrame",
    dialog,
    gate: (run, which = "login") => {
      const { frame, target } = frameOf(state, which);
      if (frame === "sameOriginFrame") {
        setPending({ run, target });
      } else {
        void run(null);
      }
    },
  };
}
