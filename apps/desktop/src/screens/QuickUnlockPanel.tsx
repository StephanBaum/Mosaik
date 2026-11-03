/**
 * Quick unlock on the unlock screen: one button per method the session offers, a waiting
 * line while the OS or the key answers, and a PIN step for security keys. Nothing prompts
 * on screen load; a click is required. The password form stays usable in every state.
 */
import { useState, type SyntheticEvent } from "react";
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

type Phase = { kind: "idle" } | { kind: "prompting"; method: QuickUnlockKind } | { kind: "pin" };

function PinForm({
  pin,
  onPin,
  onSubmit,
}: {
  pin: string;
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
  disabled,
  onStart,
}: {
  methods: readonly QuickUnlockKind[];
  disabled: boolean;
  onStart: (method: QuickUnlockKind) => void;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  return methods.map((method) => (
    <Button
      key={method}
      size="lg"
      block
      disabled={disabled}
      onClick={() => {
        onStart(method);
      }}
    >
      <Icon name={method === "biometric" ? "passkey" : "securityKey"} />
      {method === "biometric" ? t(BIOMETRIC_LABEL[platform]) : t("lock.quick.hardwareKey")}
    </Button>
  ));
}

export function QuickUnlockPanel({
  methods,
  quick,
}: {
  methods: readonly QuickUnlockKind[];
  /** Runs `unlock_quick`; resolves to the error (already shown by the screen) or `null`. */
  quick: (method: QuickUnlockKind, pin: string | null) => Promise<IpcError | null>;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const platform = usePlatform();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [pin, setPin] = useState("");

  async function start(method: QuickUnlockKind, pinValue: string | null) {
    setPhase({ kind: "prompting", method });
    const failure = await quick(method, pinValue);
    setPhase(failure !== null && PIN_CODES.includes(failure.code) ? { kind: "pin" } : { kind: "idle" });
  }

  function submitPin(event: SyntheticEvent) {
    event.preventDefault();
    const submitted = pin;
    setPin("");
    void start("hardwareKey", submitted);
  }

  function cancel() {
    setPin("");
    if (phase.kind === "prompting" && phase.method === "hardwareKey") {
      // The pending unlock call returns `cancelled`, which puts the buttons back.
      void ipc.call("hardware_key_cancel", null);
    } else {
      setPhase({ kind: "idle" });
    }
  }

  const waiting =
    phase.kind === "prompting"
      ? t(phase.method === "biometric" ? BIOMETRIC_WAITING[platform] : "lock.quick.touchKey")
      : "";
  const cancellable = phase.kind === "pin" || (phase.kind === "prompting" && phase.method === "hardwareKey");

  if (methods.length === 0) {
    return null;
  }
  return (
    <div className="form-stack">
      <div className="divider-or">{t("lock.or")}</div>
      {phase.kind === "pin" ? (
        <PinForm pin={pin} onPin={setPin} onSubmit={submitPin} />
      ) : (
        <MethodButtons
          methods={methods}
          disabled={phase.kind === "prompting"}
          onStart={(method) => {
            void start(method, null);
          }}
        />
      )}
      <p className="status" role="status">
        {waiting}
      </p>
      {cancellable && (
        <Button variant="ghost" block onClick={cancel}>
          {t("lock.quick.cancel")}
        </Button>
      )}
    </div>
  );
}
