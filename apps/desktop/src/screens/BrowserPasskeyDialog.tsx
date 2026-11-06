/**
 * The browser passkey prompt (M5, ADR-0040): a site asked to create a
 * passkey or to sign in with one, and the user decides here, in the desktop app. Nothing is signed or written before the
 * Allow: the primary button stays disabled for 1 s after the dialog opens and after the window
 * regains focus (so does the acknowledge of an exclusion). Cancel has the initial focus and Esc
 * means Cancel. No key material and no challenge reaches this window; the only secret is the
 * master password the user types when verification is needed (it is sent to Rust and cleared at
 * once). Verification can become needed while the dialog is open (the five-minute window runs
 * out): Rust then answers `reauthRequired`, the dialog switches to asking for the password and
 * moves the focus into that field.
 * The request's `requestId` is the desktop's own handle for this prompt, never the extension's;
 * a new handle remounts the prompt, so the guard timer, the selection and a typed password reset.
 */
import { useEffect, useRef, useState, type ReactNode, type SyntheticEvent } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { Chip } from "../components/Chip.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { Icon } from "../components/icons.tsx";
import { Kbd } from "../components/Kbd.tsx";
import { Select } from "../components/Select.tsx";
import { TextField } from "../components/TextField.tsx";
import { UserText } from "../components/UserText.tsx";
import { useAllowGuard } from "../hooks/useAllowGuard.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type {
  BrowserPasskeyCandidate,
  BrowserPasskeyDecision,
  BrowserPasskeyRequest,
  IpcError,
} from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { Expiry } from "./BrowserPairingDialog.tsx";
import { useWritableVaults } from "./useWritableVaults.ts";

export function BrowserPasskeyDialog() {
  const { browser } = useEvents();
  if (browser.passkey === null) {
    return null;
  }
  // Keyed by the desktop's handle: a new request remounts the prompt, whatever the page's own ID.
  return <Prompt key={browser.passkey.requestId} request={browser.passkey} />;
}

/** The host of the origin, as the URL bar shows it (the origin is an ASCII serialization). */
function hostOf(origin: string): string {
  try {
    return new URL(origin).host;
  } catch {
    return origin;
  }
}

/** Who asks, and for which site; the lock says the page is served over HTTPS. */
function Header({ request }: { request: BrowserPasskeyRequest }) {
  const { t } = useI18n();
  return (
    <div className="browser-prompt__origin">
      <p className="body">
        <span className="muted">
          <UserText>{request.pairingLabel}</UserText>
          {" · "}
        </span>
        <UserText>{hostOf(request.origin)}</UserText>
      </p>
      <Chip>
        <Icon name="lock" size="sm" />
        {t("browser.passkey.secure")}
      </Chip>
    </div>
  );
}

function CandidateLabel({ candidate }: { candidate: BrowserPasskeyCandidate }) {
  const { t } = useI18n();
  const name =
    candidate.userName !== ""
      ? candidate.userName
      : candidate.userDisplayName !== ""
        ? candidate.userDisplayName
        : t("browser.passkey.noName");
  return (
    <span className="passkey-choice__text">
      <UserText>{name}</UserText>
      <span className="caption">
        <UserText>{candidate.vaultName !== "" ? candidate.vaultName : t("vault.unnamed")}</UserText>
      </span>
    </span>
  );
}

/** The account to sign in with. The first one is selected. */
function Accounts({
  candidates,
  value,
  onChange,
}: {
  candidates: readonly BrowserPasskeyCandidate[];
  value: string;
  onChange: (itemId: string) => void;
}) {
  const { t } = useI18n();
  return (
    <fieldset className="passkey-choices">
      <legend className="field__label">{t("browser.passkey.account")}</legend>
      {candidates.map((candidate) => (
        <label key={candidate.itemId} className="passkey-choice">
          <input
            type="radio"
            name="passkey-account"
            checked={candidate.itemId === value}
            onChange={() => {
              onChange(candidate.itemId);
            }}
          />
          <CandidateLabel candidate={candidate} />
        </label>
      ))}
    </fieldset>
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

/** Create: the account the site wants a passkey for (sanitized by Rust). */
function NewAccount({ request }: { request: BrowserPasskeyRequest }) {
  const { t } = useI18n();
  const name = request.userName ?? "";
  const display = request.userDisplayName ?? "";
  const shown = name !== "" ? name : display !== "" ? display : t("browser.passkey.noName");
  return (
    <dl className="rows rows--closed">
      <Row label={t("browser.passkey.account")}>
        <UserText>{shown}</UserText>
        {name !== "" && display !== "" && display !== name && (
          <>
            {" "}
            <span className="caption">
              (<UserText>{display}</UserText>)
            </span>
          </>
        )}
      </Row>
    </dl>
  );
}

/** What the user chose; `decision` is `null` while the choice is incomplete. */
function useChoice(request: BrowserPasskeyRequest) {
  const vaults = useWritableVaults();
  const [itemId, setItemId] = useState(request.candidates[0]?.itemId ?? "");
  const [vaultId, setVaultId] = useState("");
  const chosenVault = vaultId !== "" ? vaultId : (vaults[0]?.vaultId ?? "");
  const candidate = request.candidates.find((c) => c.itemId === itemId);
  let decision: BrowserPasskeyDecision | null = null;
  if (request.excluded) {
    decision = { kind: "acknowledge" };
  } else if (request.kind === "get") {
    decision =
      candidate === undefined
        ? null
        : { kind: "assert", vaultId: candidate.vaultId, itemId: candidate.itemId };
  } else if (chosenVault !== "") {
    decision = { kind: "create", vaultId: chosenVault };
  }
  return { vaults, itemId, setItemId, chosenVault, setVaultId, decision };
}

function titleKeyOf(request: BrowserPasskeyRequest) {
  if (request.excluded) {
    return "browser.passkey.excludedTitle";
  }
  return request.kind === "get" ? "browser.passkey.getTitle" : "browser.passkey.createTitle";
}

/**
 * The master password the dialog may ask for: from the start (`needsPassword`), or once Rust
 * answers `reauthRequired` because the verification window ran out while the dialog was open.
 */
function useVerification(request: BrowserPasskeyRequest) {
  const [password, setPassword] = useState("");
  const [reauth, setReauth] = useState(false);
  const passwordRef = useRef<HTMLInputElement>(null);
  const needsTyping = (request.needsPassword || reauth) && !request.excluded;
  const askForPassword = () => {
    setReauth(true);
  };
  // The field appears while the user is on a button: the focus follows it.
  useEffect(() => {
    if (reauth) {
      passwordRef.current?.focus();
    }
  }, [reauth]);
  return { password, setPassword, needsTyping, askForPassword, passwordRef };
}

/** Sends the answer; an expired request closes the dialog, any other error keeps it open. */
function useAnswer(request: BrowserPasskeyRequest, verification: ReturnType<typeof useVerification>) {
  const ipc = useIpc();
  const { browser } = useEvents();
  const [error, setError] = useState<IpcError | null>(null);
  const { password, setPassword, needsTyping, askForPassword } = verification;

  async function answer(decision: BrowserPasskeyDecision) {
    const commits = decision.kind === "create" || decision.kind === "assert";
    const typed = commits && needsTyping ? password : "";
    const result = await ipc.call("browser_passkey_respond", {
      requestId: request.requestId,
      decision,
      password: typed === "" ? null : typed,
    });
    // The master password does not stay in this window, whatever the answer was.
    setPassword("");
    if (result.ok || result.error.code === "browserRequestExpired") {
      browser.removePasskey(request.requestId);
      return;
    }
    if (result.error.code === "reauthRequired") {
      askForPassword();
    }
    setError(result.error);
  }
  return { answer, error };
}

function Prompt({ request }: { request: BrowserPasskeyRequest }) {
  const { t } = useI18n();
  const { now, locked } = useAllowGuard();
  const choice = useChoice(request);
  const verification = useVerification(request);
  const { answer, error } = useAnswer(request, verification);
  const { password, setPassword, needsTyping, passwordRef } = verification;

  const { decision } = choice;
  const ready = decision !== null && (!needsTyping || password !== "");
  // Everything that is not a cancel waits out the Allow guard.
  const guarded =
    decision?.kind === "create" || decision?.kind === "assert" || decision?.kind === "acknowledge";
  return (
    <Dialog
      title={t(titleKeyOf(request), { rpId: request.rpId })}
      icon={
        <span className="key-tile">
          <Icon name="passkey" size="lg" />
        </span>
      }
      onClose={() => {
        void answer({ kind: "cancel" });
      }}
    >
      <form
        className="browser-prompt"
        noValidate
        onSubmit={(event: SyntheticEvent) => {
          event.preventDefault();
          if (decision !== null && ready && !(guarded && locked)) {
            void answer(decision);
          }
        }}
      >
        <Header request={request} />
        <Body request={request} choice={choice} />
        {needsTyping && (
          <>
            <p className="body muted">{t("browser.passkey.uvNote")}</p>
            <TextField
              ref={passwordRef}
              label={t("browser.passkey.password")}
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(event) => {
                setPassword(event.target.value);
              }}
            />
          </>
        )}
        <Status request={request} now={now} error={error} />
        <Actions
          request={request}
          disabled={!ready || (guarded && locked)}
          onAnswer={(each) => {
            void answer(each);
          }}
          decision={decision}
        />
      </form>
    </Dialog>
  );
}

/**
 * How long the prompt stays, how to leave it, and the last error. An exclusion is information
 * only: no countdown, just how to close it. Rust asks for the password with `reauthRequired`;
 * this dialog says so in its own words.
 */
function Status({
  request,
  now,
  error,
}: {
  request: BrowserPasskeyRequest;
  now: number;
  error: IpcError | null;
}) {
  const { t } = useI18n();
  return (
    <>
      {request.excluded ? (
        <p className="caption">
          <Kbd>{t("keys.esc")}</Kbd> {t("browser.passkey.escCloses")}
        </p>
      ) : (
        <Expiry expiresAt={request.expiresAt} now={now} label={t("browser.escCancels")} />
      )}
      <ErrorAlert
        error={error}
        message={error?.code === "reauthRequired" ? t("browser.passkey.passwordNeeded") : undefined}
      />
    </>
  );
}

/** The question, and what the user picks: the account (sign-in) or the vault (create). */
function Body({
  request,
  choice,
}: {
  request: BrowserPasskeyRequest;
  choice: ReturnType<typeof useChoice>;
}) {
  const { t } = useI18n();
  if (request.excluded) {
    return <p className="lede">{t("browser.passkey.excludedBody", { rpId: request.rpId })}</p>;
  }
  if (request.kind === "get") {
    return (
      <Accounts
        candidates={request.candidates}
        value={choice.itemId}
        onChange={choice.setItemId}
      />
    );
  }
  return (
    <>
      <p className="lede">{t("browser.passkey.createBody", { rpId: request.rpId })}</p>
      <NewAccount request={request} />
      <Select
        label={t("browser.passkey.vault")}
        options={choice.vaults.map((v) => ({ value: v.vaultId, label: v.name ?? t("vault.unnamed") }))}
        value={choice.chosenVault}
        onChange={(event) => {
          choice.setVaultId(event.target.value);
        }}
      />
    </>
  );
}

function Actions({
  request,
  disabled,
  decision,
  onAnswer,
}: {
  request: BrowserPasskeyRequest;
  disabled: boolean;
  decision: BrowserPasskeyDecision | null;
  onAnswer: (decision: BrowserPasskeyDecision) => void;
}) {
  const { t } = useI18n();
  if (request.excluded) {
    return (
      <div className="dialog__actions">
        <HoloButton
          type="submit"
          data-autofocus=""
          // Not `disabled`: the only button of the dialog must keep the focus while the guard runs.
          aria-disabled={disabled}
          onClick={(event) => {
            event.preventDefault();
            if (!disabled) {
              onAnswer({ kind: "acknowledge" });
            }
          }}
        >
          {t("browser.passkey.ok")}
        </HoloButton>
      </div>
    );
  }
  return (
    <div className="browser-prompt__actions">
      <div className="dialog__actions">
        <Button
          data-autofocus=""
          onClick={() => {
            onAnswer({ kind: "cancel" });
          }}
        >
          {t("browser.passkey.cancel")}
        </Button>
        <HoloButton type="submit" disabled={disabled || decision === null}>
          {t(request.kind === "get" ? "browser.passkey.signIn" : "browser.passkey.save")}
        </HoloButton>
      </div>
      <Button
        variant="ghost"
        className="browser-prompt__alt"
        onClick={() => {
          onAnswer({ kind: "useAnotherDevice" });
        }}
      >
        {t("browser.passkey.anotherDevice")}
      </Button>
    </div>
  );
}
