/**
 * The browser save prompt (M4, ADR-0038): the extension offers to save or update a login, and the
 * user decides here. The password never reaches this window: it stays in Rust and is shown as a
 * fixed mask. Cancel has the initial focus and Esc means Cancel; Save stays disabled for 1 s after
 * the dialog opens and after the window regains focus.
 */
import { useState, type ReactNode } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { Icon } from "../components/icons.tsx";
import { Select } from "../components/Select.tsx";
import { Switch } from "../components/Switch.tsx";
import { TextField } from "../components/TextField.tsx";
import { UserText } from "../components/UserText.tsx";
import { useAllowGuard } from "../hooks/useAllowGuard.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type {
  BrowserSaveCandidate,
  BrowserSaveDecision,
  BrowserSaveRequest,
  IpcError,
} from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { Expiry } from "./BrowserPairingDialog.tsx";
import { useWritableVaults } from "./useWritableVaults.ts";

/** What the password is shown as. */
export const PASSWORD_MASK = "••••••••";
/** The longest title (Rust refuses more). */
const MAX_TITLE_CHARS = 256;
/** The target value for "a new login" in the update choice. */
const NEW_LOGIN = "new";

export function BrowserSaveDialog() {
  const { browser } = useEvents();
  if (browser.save === null) {
    return null;
  }
  return <Prompt key={browser.save.requestId} request={browser.save} />;
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="row">
      <dt className="row__label">{label}</dt>
      <dd className="row__value">{children}</dd>
    </div>
  );
}

function Details({ request }: { request: BrowserSaveRequest }) {
  const { t } = useI18n();
  return (
    <dl className="rows rows--closed">
      <Row label={t("browser.save.site")}>
        <UserText>{request.host}</UserText>
      </Row>
      <Row label={t("browser.save.username")}>
        <UserText>{request.username ?? t("browser.save.noUsername")}</UserText>
      </Row>
      <Row label={t("browser.save.password")}>
        <span aria-label={t("browser.save.passwordHidden")} role="img">
          {PASSWORD_MASK}
        </span>
      </Row>
    </dl>
  );
}

/** The answer for what the user chose, or `null` while the choice is incomplete. */
function decisionFor(
  request: BrowserSaveRequest,
  choice: { candidate: BrowserSaveCandidate | undefined; title: string; vault: string; allowHttp: boolean },
): BrowserSaveDecision | null {
  const { candidate, title, vault, allowHttp } = choice;
  if (request.kind === "update" && candidate !== undefined) {
    return { kind: "update", vaultId: candidate.vaultId, itemId: candidate.itemId };
  }
  const titleOk = title.trim() !== "" && Array.from(title).length <= MAX_TITLE_CHARS;
  if (!titleOk || vault === "") {
    return null;
  }
  return { kind: "saveNew", vaultId: vault, title: title.trim(), allowHttp: request.insecure && allowHttp };
}

/** What the user chose; `decision` is `null` while the choice is incomplete. */
function useChoice(request: BrowserSaveRequest) {
  const vaults = useWritableVaults();
  const [target, setTarget] = useState(request.candidates[0]?.itemId ?? NEW_LOGIN);
  const [title, setTitle] = useState(request.suggestedTitle);
  const [vaultId, setVaultId] = useState("");
  const [allowHttp, setAllowHttp] = useState(false);

  const candidate = request.candidates.find((c) => c.itemId === target);
  const updating = request.kind === "update" && candidate !== undefined;
  const chosenVault = vaultId !== "" ? vaultId : (vaults[0]?.vaultId ?? "");
  const decision = decisionFor(request, { candidate, title, vault: chosenVault, allowHttp });
  return {
    vaults,
    target,
    setTarget,
    title,
    setTitle,
    chosenVault,
    setVaultId,
    allowHttp,
    setAllowHttp,
    updating,
    decision,
  };
}

function NewLoginFields({
  request,
  choice,
}: {
  request: BrowserSaveRequest;
  choice: ReturnType<typeof useChoice>;
}) {
  const { t } = useI18n();
  return (
    <>
      <TextField
        label={t("browser.save.itemTitle")}
        value={choice.title}
        maxLength={MAX_TITLE_CHARS}
        onChange={(event) => {
          choice.setTitle(event.target.value);
        }}
      />
      <Select
        label={t("browser.save.vault")}
        options={choice.vaults.map((v) => ({
          value: v.vaultId,
          label: v.name ?? t("vault.unnamed"),
        }))}
        value={choice.chosenVault}
        onChange={(event) => {
          choice.setVaultId(event.target.value);
        }}
      />
      {request.insecure && (
        <Switch
          label={t("browser.save.allowHttp")}
          hint={t("browser.save.allowHttpHint")}
          checked={choice.allowHttp}
          onChange={(event) => {
            choice.setAllowHttp(event.target.checked);
          }}
        />
      )}
    </>
  );
}

function Description({ request }: { request: BrowserSaveRequest }) {
  const { t } = useI18n();
  const key = request.kind === "update" ? "browser.save.descriptionUpdate" : "browser.save.description";
  return <p className="lede">{t(key, { browser: request.pairingLabel })}</p>;
}

function Prompt({ request }: { request: BrowserSaveRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { browser } = useEvents();
  const { now, locked } = useAllowGuard();
  const choice = useChoice(request);
  const [error, setError] = useState<IpcError | null>(null);

  async function answer(decision: BrowserSaveDecision) {
    const result = await ipc.call("browser_save_respond", {
      requestId: request.requestId,
      decision,
    });
    if (result.ok || result.error.code === "browserRequestExpired") {
      browser.removeSave(request.requestId);
    } else {
      setError(result.error);
    }
  }

  const options = [
    ...request.candidates.map((c) => ({
      value: c.itemId,
      label: t("browser.save.updateOption", { title: c.title ?? t("vault.untitled") }),
    })),
    { value: NEW_LOGIN, label: t("browser.save.newOption") },
  ];
  const { decision, updating } = choice;
  return (
    <Dialog
      title={t(request.kind === "update" ? "browser.save.titleUpdate" : "browser.save.titleNew")}
      icon={
        <span className="key-tile">
          <Icon name="key" size="lg" />
        </span>
      }
      onClose={() => {
        void answer({ kind: "cancel" });
      }}
    >
      <div className="browser-prompt">
        <Description request={request} />
        <Details request={request} />
        {request.kind === "update" && (
          <Select
            label={t("browser.save.target")}
            options={options}
            value={choice.target}
            onChange={(event) => {
              choice.setTarget(event.target.value);
            }}
          />
        )}
        {!updating && <NewLoginFields request={request} choice={choice} />}
        <Expiry expiresAt={request.expiresAt} now={now} label={t("browser.escCancels")} />
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button
            data-autofocus=""
            onClick={() => {
              void answer({ kind: "cancel" });
            }}
          >
            {t("browser.save.cancel")}
          </Button>
          <HoloButton
            disabled={locked || decision === null}
            onClick={() => {
              if (decision !== null) {
                void answer(decision);
              }
            }}
          >
            {t(updating ? "browser.save.update" : "browser.save.saveNew")}
          </HoloButton>
        </div>
      </div>
    </Dialog>
  );
}
