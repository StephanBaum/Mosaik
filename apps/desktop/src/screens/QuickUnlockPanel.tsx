/**
 * Quick unlock on the unlock screen: one button per method the session offers, a waiting
 * line while the OS or the key answers, and a PIN step for security keys. Nothing prompts
 * on screen load; a click is required. The password form stays usable in every state,
 * except that it cannot be sent while a method waits. This panel shows its own errors next
 * to the PIN field and the buttons.
 */
import { useEffect, useId, useRef, useState, type RefObject, type SyntheticEvent } from "react";
import { Button } from "../components/Button.tsx";
import { Icon } from "../components/icons.tsx";
import { PasswordField } from "../components/PasswordField.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { IpcError, QuickUnlockKind } from "../ipc/contract/index.ts";
import { usePlatform, type Platform } from "../platform/platform.ts";

const BIOMETRIC_LABEL = {
  windows: "lock.quick.biometricWindows",
  mac: "lock.quick.biometricMac",
  linux: "lock.quick.biometricLinux",
} as const satisfies Record<Platform, string>;

const BIOMETRIC_WAITING = {
  windows: "lock.quick.waitingWindows",
  mac: "lock.quick.waitingMac",
  linux: "lock.quick.waitingWindows",
} as const satisfies Record<Platform, string>;

/** The security key wants a PIN (first ask, or the last one was wrong). */
const PIN_CODES: readonly string[] = ["hardwareKeyPinRequired", "hardwareKeyPinInvalid"];

/** After these the buttons are gone: the screen reads the session again and asks for the password. */
export const REREAD_CODES: readonly string[] = [
  "quickUnlockExpired",
  "quickUnlockInvalidated",
  "passwordRequired",
];

type Phase = { kind: "idle" } | { kind: "prompting"; method: QuickUnlockKind } | { kind: "pin" };

function PinForm({
  pin,
  error,
  onPin,
  onSubmit,
}: {
  pin: string;
  error: string | undefined;
  onPin: (pin: string) => void;
  onSubmit: (event: SyntheticEvent) => void;
}) {
  const { t } = useI18n();
  return (
    <form className="form-stack" onSubmit={onSubmit} noValidate>
      <PasswordField
        size="lg"
        label={t("lock.quick.pin")}
        autoComplete="off"
        autoFocus
        error={error}
        value={pin}
        onChange={(event) => {
          onPin(event.target.value);
        }}
      />
      <Button type="submit" size="lg" block>
        {t("lock.quick.pinSubmit")}
      </Button>
    </form>
  );
}

function MethodButtons({
  methods,
  busy,
  describedBy,
  onStart,
}: {
  methods: readonly QuickUnlockKind[];
  busy: boolean;
  describedBy: string | undefined;
  onStart: (method: QuickUnlockKind) => void;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  return methods.map((method) => (
    <Button
      key={method}
      size="lg"
      block
      data-method={method}
      // Not `disabled`: a disabled button drops the focus the user just gave it.
      aria-disabled={busy ? true : undefined}
      aria-describedby={describedBy}
      className={busy ? "button--busy" : undefined}
      onClick={() => {
        if (!busy) {
          onStart(method);
        }
      }}
    >
      <Icon name={method === "biometric" ? "passkey" : "securityKey"} />
      {method === "biometric" ? t(BIOMETRIC_LABEL[platform]) : t("lock.quick.hardwareKey")}
    </Button>
  ));
}

/** What a failed `unlock_quick` means for this panel. */
function classify(failure: IpcError | null): { askPin: boolean; show: boolean; refocus: boolean } {
  const code = failure?.code ?? null;
  const askPin = code !== null && PIN_CODES.includes(code);
  const reread = code !== null && REREAD_CODES.includes(code);
  // A deliberate cancel, the PIN prompt and the screen's own messages are not errors here.
  const quiet = code === null || code === "cancelled" || code === "hardwareKeyPinRequired" || reread;
  return { askPin, show: !quiet, refocus: !askPin && !reread };
}

type Quick = (method: QuickUnlockKind, pin: string | null) => Promise<IpcError | null>;

/** The panel's state: which step it is in, the PIN, and its own error. */
function useQuickFlow(
  quick: Quick,
  root: RefObject<HTMLDivElement | null>,
  clearSignal: number | undefined,
  onPrompting: ((prompting: boolean) => void) | undefined,
) {
  const ipc = useIpc();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [pin, setPin] = useState("");
  const [error, setError] = useState<IpcError | null>(null);
  const focus = (selector: string) => {
    requestAnimationFrame(() => {
      root.current?.querySelector<HTMLElement>(selector)?.focus();
    });
  };

  // The password form was used: an old error here no longer applies.
  const [signal, setSignal] = useState(clearSignal);
  if (signal !== clearSignal) {
    setSignal(clearSignal);
    setError(null);
  }

  // A security key waits with a Cancel button: move focus there, off the button just pressed.
  const keyPrompting = phase.kind === "prompting" && phase.method === "hardwareKey";
  useEffect(() => {
    if (keyPrompting) {
      root.current?.querySelector<HTMLElement>("[data-cancel]")?.focus();
    }
  }, [keyPrompting, root]);

  async function start(method: QuickUnlockKind, pinValue: string | null) {
    setPhase({ kind: "prompting", method });
    setError(null);
    onPrompting?.(true);
    const failure = await quick(method, pinValue);
    onPrompting?.(false);
    const outcome = classify(failure);
    setPhase(outcome.askPin ? { kind: "pin" } : { kind: "idle" });
    setError(outcome.show ? failure : null);
    if (outcome.refocus) {
      focus(`[data-method="${method}"]`); // back to the button the user used
    }
  }

  function cancel() {
    setPin("");
    setError(null);
    if (keyPrompting) {
      // The pending unlock call returns `cancelled`, which puts the buttons back.
      void ipc.call("hardware_key_cancel", null);
    } else {
      setPhase({ kind: "idle" });
      focus('[data-method="hardwareKey"]');
    }
  }

  function submitPin(event: SyntheticEvent) {
    event.preventDefault();
    const submitted = pin;
    setPin("");
    void start("hardwareKey", submitted);
  }
  return { phase, pin, setPin, error, start, cancel, submitPin, cancellable: phase.kind === "pin" || keyPrompting };
}

export function QuickUnlockPanel({
  methods,
  quick,
  clearSignal,
  onPrompting,
}: {
  methods: readonly QuickUnlockKind[];
  /** Runs `unlock_quick`; resolves to the error or `null`. Codes in `REREAD_CODES` are shown by the screen. */
  quick: Quick;
  /** Changes when the password form was used. */
  clearSignal?: number;
  /** Whether a method is waiting for the OS or the key. */
  onPrompting?: (prompting: boolean) => void;
}) {
  const { t, errorMessage } = useI18n();
  const platform = usePlatform();
  const root = useRef<HTMLDivElement>(null);
  const errorId = useId();
  const flow = useQuickFlow(quick, root, clearSignal, onPrompting);
  const { phase } = flow;
  const message = flow.error === null ? undefined : errorMessage(flow.error);

  if (methods.length === 0) {
    return null;
  }
  return (
    <div ref={root} className="form-stack">
      <div className="divider-or">{t("lock.or")}</div>
      {phase.kind === "pin" ? (
        <PinForm pin={flow.pin} error={message} onPin={flow.setPin} onSubmit={flow.submitPin} />
      ) : (
        <MethodButtons
          methods={methods}
          busy={phase.kind === "prompting"}
          describedBy={message === undefined ? undefined : errorId}
          onStart={(method) => {
            void flow.start(method, null);
          }}
        />
      )}
      {phase.kind !== "pin" && (
        <p id={errorId} className="alert" role="alert">
          {message ?? ""}
        </p>
      )}
      <p className="status caption" role="status">
        {phase.kind === "prompting"
          ? t(phase.method === "biometric" ? BIOMETRIC_WAITING[platform] : "lock.quick.touchKey")
          : ""}
      </p>
      {phase.kind === "prompting" && <span className="progress" aria-hidden="true" />}
      {flow.cancellable && (
        <Button data-cancel="" variant="ghost" block onClick={flow.cancel}>
          {t("lock.quick.cancel")}
        </Button>
      )}
    </div>
  );
}
