/**
 * The browser step of a security key (M9): the six-digit match code in large type, "Open the
 * browser page again", Cancel, and the expiry. Rust opened the server's page in the system
 * browser; the user types this code there and touches the key. The step polls its status
 * command every two seconds. Focus moves to the heading when the step appears, the code is
 * spoken digit by digit, and a live region reports the status.
 */
import { useEffect, useRef, useState, type HTMLAttributes, type Ref } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type {
  HandoffState,
  HandoffStep,
  IpcError,
  IpcResult,
  UnixMillis,
} from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";

/** A step heading that takes the level of the screen it is on (h2 under a page h1, h4 in a card). */
export function Heading({
  level,
  ref,
  ...rest
}: HTMLAttributes<HTMLHeadingElement> & { level: 2 | 4; ref?: Ref<HTMLHeadingElement> }) {
  const Tag = level === 2 ? "h2" : "h4";
  return <Tag ref={ref} {...rest} />;
}

/** How often the status is asked. */
export const POLL_MS = 2000;

/** What a status call answers; the account commands also say when the next step started. */
export interface PollAnswer {
  state: HandoffState;
  step?: HandoffStep;
  matchCode?: string | null;
  expiresAt?: UnixMillis | null;
}

/** The step the user sees: its heading and its code. */
export interface StepView {
  step: HandoffStep;
  matchCode: string;
  expiresAt: UnixMillis;
}

interface Callbacks {
  poll: () => Promise<IpcResult<PollAnswer>>;
  onView: ((view: StepView) => void) | undefined;
  onDone: () => void;
}

/** Asks the status every `POLL_MS` while the step waits; ends the wait on any other state. */
function useHandoffPolling(
  callbacks: Callbacks,
  state: HandoffState,
  setState: (state: HandoffState) => void,
  setError: (error: IpcError | null) => void,
) {
  const latest = useRef(callbacks);
  useEffect(() => {
    latest.current = callbacks;
  });
  useEffect(() => {
    if (state !== "waiting") {
      return undefined;
    }
    let alive = true;
    let busy = false;
    const timer = setInterval(() => {
      if (busy) {
        return;
      }
      busy = true;
      void latest.current.poll().then((result) => {
        busy = false;
        if (!alive) {
          return;
        }
        setError(result.ok ? null : result.error);
        if (!result.ok) {
          return;
        }
        const answer = result.value;
        if (answer.state === "done") {
          setState("done");
          latest.current.onDone();
        } else if (answer.state !== "waiting") {
          setState(answer.state);
        } else if (answer.step !== undefined && answer.matchCode && answer.expiresAt) {
          latest.current.onView?.({
            step: answer.step,
            matchCode: answer.matchCode,
            expiresAt: answer.expiresAt,
          });
        }
      });
    }, POLL_MS);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [state, setState, setError]);
}

/** The code to type in the browser, and when the step runs out. */
function HandoffCode({ view }: { view: StepView }) {
  const { t, locale } = useI18n();
  const time = new Intl.DateTimeFormat(locale, { timeStyle: "short" }).format(view.expiresAt);
  return (
    <>
      <p className="field__hint">{t("settings.sync.key.wait.lead")}</p>
      <div className="handoff__code-block">
        <p className="caption">{t("settings.sync.key.wait.codeLabel")}</p>
        {/* A screen reader says "4 8 2 ..." instead of "four hundred eighty-two thousand". */}
        <p className="handoff__code mono">
          <span aria-hidden="true">{view.matchCode}</span>
          <span className="visually-hidden">
            {t("settings.sync.key.wait.codeSpoken", { digits: view.matchCode.split("").join(" ") })}
          </span>
        </p>
        <p className="caption">{t("settings.sync.key.wait.expires", { time })}</p>
      </div>
    </>
  );
}

export function SecurityKeyStep({
  view,
  headingLevel = 4,
  poll,
  onView,
  onDone,
  onCancel,
  onRetry,
}: {
  view: StepView;
  /** The level of the step heading: 4 under a card in Settings, 2 on a screen with its own h1. */
  headingLevel?: 2 | 4;
  /** Asks Rust where the step stands. */
  poll: () => Promise<IpcResult<PollAnswer>>;
  /** The next step started (an account flow that signed in first). */
  onView?: (view: StepView) => void;
  onDone: () => void;
  onCancel: () => void;
  /** After an expired or failed step: start again. */
  onRetry: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const heading = useRef<HTMLHeadingElement>(null);
  const [state, setState] = useState<HandoffState>("waiting");
  const [error, setError] = useState<IpcError | null>(null);

  // Focus the heading once, and again for the next step of the same flow.
  useEffect(() => {
    heading.current?.focus();
  }, [view.step]);
  useHandoffPolling({ poll, onView, onDone }, state, setState, setError);

  const ended = state === "expired" || state === "failed";
  const statusKey = {
    waiting: "settings.sync.key.wait.waiting",
    done: "settings.sync.key.wait.done",
    expired: "settings.sync.key.wait.expired",
    failed: "settings.sync.key.wait.failed",
  } as const;
  return (
    <section className="stack handoff" aria-labelledby="handoff-heading">
      <Heading level={headingLevel} id="handoff-heading" className="body" ref={heading} tabIndex={-1}>
        {t(`settings.sync.key.wait.${view.step}`)}
      </Heading>
      {!ended && <HandoffCode view={view} />}
      <p className="status" role="status">
        {t(statusKey[state])}
      </p>
      <ErrorAlert error={error} />
      <div className="actions">
        {ended ? (
          <Button onClick={onRetry}>{t("settings.sync.key.wait.retry")}</Button>
        ) : (
          <Button
            onClick={() => {
              void ipc.call("sync_webauthn_reopen", null);
            }}
          >
            {t("settings.sync.key.wait.reopen")}
          </Button>
        )}
        <Button variant="ghost" onClick={onCancel}>
          {t("action.cancel")}
        </Button>
      </div>
    </section>
  );
}
