/**
 * The onboarding state machine. The state keeps only what the later steps need: the
 * session, and the Emergency Kit (with the Secret Key) until the kit is verified. The
 * account object the backend returned is never stored whole.
 */
import type { CreatedAccount, SessionState } from "../../ipc/contract/index.ts";

export type Step = "kit" | "verify" | "import" | "importing" | "done";

export interface FlowState {
  step: Step;
  session: SessionState | null;
  /** The kit with the Secret Key; `null` before the account exists and after `verified`. */
  kit: CreatedAccount["emergencyKit"] | null;
}

export type FlowAction =
  | { type: "created"; account: CreatedAccount }
  | { type: "kitSaved" }
  | { type: "verified" }
  | { type: "importStart" }
  | { type: "skip" }
  | { type: "done" };

export const initialFlow: FlowState = { step: "kit", session: null, kit: null };

export function flowReducer(state: FlowState, action: FlowAction): FlowState {
  switch (action.type) {
    case "created":
      return { step: "kit", session: action.account.session, kit: action.account.emergencyKit };
    case "kitSaved":
      return { ...state, step: "verify" };
    case "verified":
      // The Secret Key leaves the webview's state as soon as the kit is verified.
      return { ...state, step: "import", kit: null };
    case "importStart":
      return { ...state, step: "importing" };
    case "skip":
    case "done":
      return { ...state, step: "done" };
  }
}
