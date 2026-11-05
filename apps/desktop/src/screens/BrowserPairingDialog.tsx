/**
 * The browser pairing prompt (M4, ADR-0036): the extension shows the same code, the user checks
 * that they match and that they just clicked Connect, names the browser and allows. Deny has the
 * initial focus and Esc means Deny; Allow stays disabled for 1 s after the dialog opens and after
 * the window regains focus.
 */
import { useEffect, useRef, useState } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { CountdownRing } from "../components/CountdownRing.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { Icon } from "../components/icons.tsx";
import { Kbd } from "../components/Kbd.tsx";
import { TextField } from "../components/TextField.tsx";
import { useAllowGuard } from "../hooks/useAllowGuard.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type {
  BrowserPairingRequest,
  IpcError,
  PairingDecision,
} from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";

/** The longest pairing label (Rust refuses more). */
export const MAX_LABEL_CHARS = 64;
/** The countdown is announced to screen readers once, when this many seconds remain. */
const ANNOUNCE_AT_SECONDS = 10;

/** `4F7K2Q` as `4F7-K2Q`. */
export function formatCode(code: string): string {
  return code.length === 6 ? `${code.slice(0, 3)}-${code.slice(3)}` : code;
}

export function BrowserPairingDialog() {
  const { browser } = useEvents();
  if (browser.pairing === null) {
    return null;
  }
  // A new request restarts the dialog (and its guard).
  return <Prompt key={browser.pairing.requestId} request={browser.pairing} />;
}

/** The countdown text for a live region: empty until `ANNOUNCE_AT_SECONDS`, then fixed. */
export function useAnnouncement(seconds: number): string {
  const { t } = useI18n();
  const announced = useRef(false);
  const [announcement, setAnnouncement] = useState("");
  useEffect(() => {
    if (seconds <= ANNOUNCE_AT_SECONDS && !announced.current) {
      announced.current = true;
      setAnnouncement(t("browser.expires", { seconds }));
    }
  }, [seconds, t]);
  return announcement;
}

/** The ring, the countdown text and the Esc hint, with the live region for screen readers. */
export function Expiry({ expiresAt, now, label }: { expiresAt: number; now: number; label: string }) {
  const { t } = useI18n();
  const [total] = useState(() => Math.max(1, Math.ceil((expiresAt - Date.now()) / 1000)));
  const seconds = Math.max(0, Math.ceil((expiresAt - now) / 1000));
  const announcement = useAnnouncement(seconds);
  return (
    <>
      <div className="browser-prompt__expiry caption">
        <CountdownRing remaining={seconds} total={total} />
        <p className="browser-prompt__expiry-text">{t("browser.expires", { seconds })}</p>
        <span>
          <Kbd>{t("keys.esc")}</Kbd> {label}
        </span>
      </div>
      <p role="status" className="visually-hidden">
        {announcement}
      </p>
    </>
  );
}

function CodeBox({ request, browserName }: { request: BrowserPairingRequest; browserName: string }) {
  const { t } = useI18n();
  return (
    <div className="browser-prompt__code-box">
      <span className="caption">{t("browser.pair.code")}</span>
      <p className="browser-prompt__code" translate="no">
        {formatCode(request.code)}
      </p>
      <span className="caption">
        {t("browser.pair.browser")}: {browserName}
      </span>
    </div>
  );
}

function Prompt({ request }: { request: BrowserPairingRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { browser } = useEvents();
  const { now, locked } = useAllowGuard();
  const browserName = t(`browser.hint.${request.hint}`);
  const [label, setLabel] = useState(browserName);
  const [error, setError] = useState<IpcError | null>(null);

  async function answer(decision: PairingDecision) {
    const result = await ipc.call("browser_pairing_respond", {
      requestId: request.requestId,
      decision,
      label: decision === "allow" ? label.trim() : null,
    });
    if (result.ok || result.error.code === "browserRequestExpired") {
      browser.removePairing(request.requestId);
    } else {
      setError(result.error);
    }
  }

  const labelOk = label.trim() !== "" && Array.from(label).length <= MAX_LABEL_CHARS;
  return (
    <Dialog
      title={t("browser.pair.title")}
      icon={
        <span className="key-tile">
          <Icon name="shield" size="lg" />
        </span>
      }
      onClose={() => {
        void answer("deny");
      }}
    >
      <div className="browser-prompt">
        <p className="lede">{t("browser.pair.description")}</p>
        <CodeBox request={request} browserName={browserName} />
        <TextField
          label={t("browser.pair.label")}
          hint={t("browser.pair.labelHint")}
          value={label}
          maxLength={MAX_LABEL_CHARS}
          onChange={(event) => {
            setLabel(event.target.value);
          }}
        />
        <Expiry expiresAt={request.expiresAt} now={now} label={t("browser.escDenies")} />
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button
            data-autofocus=""
            onClick={() => {
              void answer("deny");
            }}
          >
            {t("browser.pair.deny")}
          </Button>
          <HoloButton
            disabled={locked || !labelOk}
            onClick={() => {
              void answer("allow");
            }}
          >
            {t("browser.pair.allow")}
          </HoloButton>
        </div>
      </div>
    </Dialog>
  );
}
