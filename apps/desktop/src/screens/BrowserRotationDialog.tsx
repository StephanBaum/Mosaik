/**
 * The browser's password-change dialog (M7b, ADR-0068): the extension starts a change (`begin`),
 * reports how it went (`result`) or asks to add a related address (`addOrigin`), and the user
 * decides here. Nothing is stored or changed before the answer. The generated password stays in
 * Rust: it is shown as a fixed mask, and "Show" fetches it for a few seconds; "Generate another"
 * asks Rust for a new one (it keeps the page's password rules). Cancel has the initial focus and
 * Esc means Cancel; the answers that write (use, confirm, add) stay disabled for 1 s after the
 * dialog opens, after the window regains focus and after a new password was generated. "Keep the
 * old password" is never delayed: it changes nothing that matters.
 * The dialog follows the events itself (`browserRotationRequested` and `...Closed`) and closes on
 * a lock, so it needs nothing from the shared prompt state.
 */
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "../components/Button.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { Icon } from "../components/icons.tsx";
import { UserText } from "../components/UserText.tsx";
import { useAllowGuard } from "../hooks/useAllowGuard.ts";
import { useIpcEvent } from "../hooks/useIpcEvent.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type {
  BrowserRotationDecision,
  BrowserRotationRequest,
  IpcError,
} from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { Expiry } from "./BrowserPairingDialog.tsx";

/** What the password is shown as. */
export const ROTATION_MASK = "••••••••";

export function BrowserRotationDialog() {
  const [open, setOpen] = useState<BrowserRotationRequest | null>(null);
  useIpcEvent((event) => {
    switch (event.type) {
      case "browserRotationRequested":
        setOpen(event.request);
        break;
      case "browserRotationClosed":
        setOpen((current) => (current?.requestId === event.requestId ? null : current));
        break;
      case "locked":
        setOpen(null);
        break;
      default:
        break;
    }
  });
  if (open === null) {
    return null;
  }
  return (
    <Prompt
      key={open.requestId}
      request={open}
      close={() => {
        setOpen((current) => (current?.requestId === open.requestId ? null : current));
      }}
    />
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="row">
      <dt className="row__label">{label}</dt>
      <dd className="row__value">{children}</dd>
    </div>
  );
}

function hostText(request: BrowserRotationRequest): string {
  return request.unicodeHost ?? request.host;
}

function useHeading(request: BrowserRotationRequest): string {
  const { t } = useI18n();
  const host = hostText(request);
  const title = request.title ?? t("credentialRotation.dialog.untitled");
  if (request.kind === "begin") {
    return t("credentialRotation.dialog.beginTitle");
  }
  if (request.kind === "addOrigin") {
    return t("credentialRotation.addOrigin.title", { host, title });
  }
  return request.outcome === "failed"
    ? t("credentialRotation.dialog.failedTitle")
    : t("credentialRotation.dialog.resultTitle", { host });
}

function Description({ request, browser }: { request: BrowserRotationRequest; browser: string }) {
  const { t } = useI18n();
  const host = hostText(request);
  const title = request.title ?? t("credentialRotation.dialog.untitled");
  const key =
    request.kind === "begin"
      ? "credentialRotation.dialog.beginBody"
      : request.kind === "addOrigin"
        ? "credentialRotation.addOrigin.body"
        : request.outcome === "failed"
          ? "credentialRotation.dialog.failedBody"
          : "credentialRotation.dialog.resultBody";
  return <p className="lede">{t(key, { browser, host, title })}</p>;
}

/** The generated password: a mask, "Show" for a few seconds, "Generate another". */
function GeneratedPassword({
  request,
  regenerate,
  setError,
}: {
  request: BrowserRotationRequest;
  regenerate: () => Promise<void>;
  setError: (error: IpcError | null) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [shown, setShown] = useState<string | null>(null);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(
    () => () => {
      clearTimeout(hideTimer.current);
    },
    [],
  );

  async function show() {
    const result = await ipc.call("browser_rotation_reveal", { requestId: request.requestId });
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setError(null);
    setShown(result.value.value);
    clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      setShown(null);
    }, result.value.hideAfterSeconds * 1000);
  }

  return (
    <Row label={t("credentialRotation.dialog.password")}>
      {shown === null ? (
        <span aria-label={t("credentialRotation.dialog.hidden")} role="img">
          {ROTATION_MASK}
        </span>
      ) : (
        <code className="mono">{shown}</code>
      )}{" "}
      <Button
        onClick={() => {
          if (shown === null) {
            void show();
          } else {
            clearTimeout(hideTimer.current);
            setShown(null);
          }
        }}
      >
        {t(shown === null ? "credentialRotation.dialog.show" : "credentialRotation.dialog.hide")}
      </Button>{" "}
      <Button
        onClick={() => {
          void regenerate();
        }}
      >
        {t("credentialRotation.dialog.regenerate")}
      </Button>
    </Row>
  );
}

/** The decision of the main action for this request, and the label of its button. */
function mainAction(request: BrowserRotationRequest): {
  decision: BrowserRotationDecision;
  labelKey: "credentialRotation.dialog.allow" | "credentialRotation.dialog.confirm" | "credentialRotation.addOrigin.add";
} | null {
  if (request.kind === "begin") {
    return { decision: { kind: "allow" }, labelKey: "credentialRotation.dialog.allow" };
  }
  if (request.kind === "addOrigin") {
    return { decision: { kind: "addOrigin" }, labelKey: "credentialRotation.addOrigin.add" };
  }
  return request.outcome === "succeeded"
    ? { decision: { kind: "confirm" }, labelKey: "credentialRotation.dialog.confirm" }
    : null;
}

/** Which attempt a result closes: when it started, how far it got, how many others are open. */
function AttemptLine({ request }: { request: BrowserRotationRequest }) {
  const { t } = useI18n();
  const { attempt } = request;
  if (request.kind !== "result" || attempt === null) {
    return null;
  }
  return (
    <p className="caption">
      {t("credentialRotation.dialog.attempt", {
        at: new Date(attempt.startedAt),
        stage: t(
          attempt.stage === "delivered"
            ? "credentialRotation.dialog.stageDelivered"
            : "credentialRotation.dialog.stagePrepared",
        ),
        n: attempt.otherOpen,
      })}
    </p>
  );
}

/** The site, the login and, for a `begin`, the generated password. */
function Details({
  request,
  regenerate,
  setError,
}: {
  request: BrowserRotationRequest;
  regenerate: () => Promise<void>;
  setError: (error: IpcError | null) => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <dl className="rows rows--closed">
        <Row label={t("credentialRotation.dialog.site")}>
          <UserText>{hostText(request)}</UserText>
        </Row>
        {request.title !== null && (
          <Row label={t("credentialRotation.dialog.login")}>
            <UserText>{request.title}</UserText>
          </Row>
        )}
        {request.kind === "begin" && (
          <GeneratedPassword request={request} setError={setError} regenerate={regenerate} />
        )}
      </dl>
      <AttemptLine request={request} />
      {request.kind === "addOrigin" && request.unicodeHost !== null && (
        <p className="caption" role="note">
          {t("credentialRotation.addOrigin.idn", { unicode: request.unicodeHost })}
        </p>
      )}
    </>
  );
}

function Body({
  request,
  announcement,
  answer,
  regenerate,
}: {
  request: BrowserRotationRequest;
  announcement: string;
  answer: (decision: BrowserRotationDecision) => Promise<IpcError | null>;
  regenerate: () => Promise<IpcError | null>;
}) {
  const { t } = useI18n();
  const { now, locked } = useAllowGuard();
  const [error, setError] = useState<IpcError | null>(null);
  const main = mainAction(request);
  const cancelKey =
    request.kind === "addOrigin" ? "credentialRotation.addOrigin.cancel" : "credentialRotation.dialog.cancel";

  async function decide(decision: BrowserRotationDecision) {
    setError(await answer(decision));
  }

  return (
    <div className="browser-prompt">
      <Description request={request} browser={request.pairingLabel} />
      <Details
        request={request}
        setError={setError}
        regenerate={async () => {
          setError(await regenerate());
        }}
      />
      <Expiry expiresAt={request.expiresAt} now={now} label={t("browser.escCancels")} />
      <p role="status" className="visually-hidden">
        {announcement}
      </p>
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button
          data-autofocus=""
          onClick={() => {
            void decide({ kind: "cancel" });
          }}
        >
          {t(cancelKey)}
        </Button>
        {request.kind === "result" && (
          <Button
            onClick={() => {
              void decide({ kind: "keepOld" });
            }}
          >
            {t("credentialRotation.dialog.keepOld")}
          </Button>
        )}
        {main !== null && (
          <HoloButton
            disabled={locked}
            onClick={() => {
              void decide(main.decision);
            }}
          >
            {t(main.labelKey)}
          </HoloButton>
        )}
      </div>
    </div>
  );
}

function Prompt({ request, close }: { request: BrowserRotationRequest; close: () => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [generation, setGeneration] = useState(0);
  const heading = useHeading(request);
  const [announcement, setAnnouncement] = useState("");

  async function answer(decision: BrowserRotationDecision): Promise<IpcError | null> {
    const result = await ipc.call("browser_rotation_respond", {
      requestId: request.requestId,
      decision,
    });
    if (result.ok || result.error.code === "browserRequestExpired") {
      close();
      return null;
    }
    return result.error;
  }

  async function regenerate(): Promise<IpcError | null> {
    const result = await ipc.call("browser_rotation_respond", {
      requestId: request.requestId,
      decision: { kind: "regenerate" },
    });
    if (!result.ok) {
      return result.error.code === "browserRequestExpired" ? null : result.error;
    }
    // A new password restarts the Allow guard: the body mounts again, hidden.
    setGeneration((n) => n + 1);
    setAnnouncement(t("credentialRotation.dialog.regenerated"));
    return null;
  }

  return (
    <Dialog
      title={heading}
      icon={
        <span className="key-tile">
          <Icon name="key" size="lg" />
        </span>
      }
      onClose={() => {
        void answer({ kind: "cancel" });
      }}
    >
      <Body
        key={generation}
        request={request}
        announcement={announcement}
        answer={answer}
        regenerate={regenerate}
      />
    </Dialog>
  );
}
