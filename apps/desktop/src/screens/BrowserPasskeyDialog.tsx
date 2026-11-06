/**
 * The browser passkey prompt (M5, ADR-0040): a site asked to create a passkey or to sign in with
 * one, and the user decides here, in the desktop app. Nothing is signed or written before the
 * Allow: the primary button stays disabled for 1 s after the dialog opens and after the window
 * regains focus. Cancel has the initial focus and Esc means Cancel. No key material and no
 * challenge reaches this window; the only secret is the master password the user types when the
 * site asked for user verification (it is sent to Rust and cleared at once).
 */
import { useState, type ReactNode, type SyntheticEvent } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { Chip } from "../components/Chip.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { Icon } from "../components/icons.tsx";
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
      <p className="caption">
        <UserText>{request.pairingLabel}</UserText>
        {" · "}
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

function Prompt({ request }: { request: BrowserPasskeyRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { browser } = useEvents();
  const { now, locked } = useAllowGuard();
  const choice = useChoice(request);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<IpcError | null>(null);

  async function answer(decision: BrowserPasskeyDecision) {
    const commits = decision.kind === "create" || decision.kind === "assert";
    const typed = commits && request.needsPassword ? password : "";
    const result = await ipc.call("browser_passkey_respond", {
      requestId: request.requestId,
      decision,
      password: typed === "" ? null : typed,
    });
    // The master password does not stay in this window, whatever the answer was.
    setPassword("");
    if (result.ok || result.error.code === "browserRequestExpired") {
      browser.removePasskey(request.requestId);
    } else {
      setError(result.error);
    }
  }

  const { decision } = choice;
  const needsTyping = request.needsPassword && !request.excluded;
  const ready = decision !== null && (!needsTyping || password !== "");
  const commits = decision?.kind === "create" || decision?.kind === "assert";
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
          if (decision !== null && ready && !(commits && locked)) {
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
        <Expiry expiresAt={request.expiresAt} now={now} label={t("browser.escCancels")} />
        <ErrorAlert error={error} />
        <Actions
          request={request}
          disabled={!ready || (commits && locked)}
          onAnswer={(each) => {
            void answer(each);
          }}
          decision={decision}
        />
      </form>
    </Dialog>
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
          onClick={(event) => {
            event.preventDefault();
            onAnswer({ kind: "acknowledge" });
          }}
        >
          {t("browser.passkey.ok")}
        </HoloButton>
      </div>
    );
  }
  return (
    <div className="dialog__actions">
      <Button
        data-autofocus=""
        onClick={() => {
          onAnswer({ kind: "cancel" });
        }}
      >
        {t("browser.passkey.cancel")}
      </Button>
      <Button
        onClick={() => {
          onAnswer({ kind: "useAnotherDevice" });
        }}
      >
        {t("browser.passkey.anotherDevice")}
      </Button>
      <HoloButton type="submit" disabled={disabled || decision === null}>
        {t(request.kind === "get" ? "browser.passkey.signIn" : "browser.passkey.save")}
      </HoloButton>
    </div>
  );
}
