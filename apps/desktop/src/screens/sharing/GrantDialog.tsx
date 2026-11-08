/**
 * Allow emergency access (CRYPTO_SPEC §8.4): who (a verified contact), what (all vaults or some),
 * the waiting time (the server keeps it; seven days by default) and the master password. A grant
 * covers vault keys only, never the account.
 */
import { useEffect, useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { Select } from "../../components/Select.tsx";
import { TextField } from "../../components/TextField.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import {
  defaultWaitDays,
  waitDays,
  type AccountId,
  type EmergencyScopeView,
  type IpcError,
  type VaultSummary,
} from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "../sync/hooks.ts";
import { vaultLabel } from "../vault/Sidebar.tsx";
import { ContactVerifier } from "./ContactVerifier.tsx";
import { StepHeading } from "./StepHeading.tsx";

type Stage =
  | { kind: "email" }
  | { kind: "verify"; email: string }
  | { kind: "options"; accountId: AccountId; email: string };

/** The vaults that can be named: the ones this account holds and a grant can seal. */
function useGrantableVaults(): VaultSummary[] {
  const ipc = useIpc();
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  useEffect(() => {
    let current = true;
    void ipc.call("list_vaults", null).then((result) => {
      if (current && result.ok) {
        setVaults(result.value.filter((v) => v.protection === "accountKey" && v.available));
      }
    });
    return () => {
      current = false;
    };
  }, [ipc]);
  return vaults;
}

/** Which vaults a grant covers: all of them (new ones included), or a chosen few. */
function ScopeFields({
  all,
  chosen,
  onAll,
  onChosen,
}: {
  all: boolean;
  chosen: ReadonlySet<string>;
  onAll: (all: boolean) => void;
  onChosen: (chosen: ReadonlySet<string>) => void;
}) {
  const { t } = useI18n();
  const vaults = useGrantableVaults();
  return (
    <fieldset className="fieldset">
      <legend className="fieldset__legend">{t("emergency.grant.scope")}</legend>
      <label className="check">
        <input
          type="radio"
          name="grant-scope"
          checked={all}
          onChange={() => {
            onAll(true);
          }}
        />
        {t("emergency.grant.scope.all")}
      </label>
      <label className="check">
        <input
          type="radio"
          name="grant-scope"
          checked={!all}
          onChange={() => {
            onAll(false);
          }}
        />
        {t("emergency.grant.scope.selected")}
      </label>
      {!all && (
        <div className="stack">
          {vaults.map((vault) => (
            <label key={vault.vaultId} className="check">
              <input
                type="checkbox"
                checked={chosen.has(vault.vaultId)}
                onChange={(event) => {
                  const next = new Set(chosen);
                  if (event.target.checked) {
                    next.add(vault.vaultId);
                  } else {
                    next.delete(vault.vaultId);
                  }
                  onChosen(next);
                }}
              />
              <UserText>{vaultLabel(vault, (key) => t(key))}</UserText>
            </label>
          ))}
          {chosen.size === 0 && <p className="field__hint">{t("emergency.grant.scope.pick")}</p>}
        </div>
      )}
    </fieldset>
  );
}

/** The master password (and the second-factor code) of the fresh login a grant needs. */
function CredentialFields({
  totp,
  password,
  code,
  onPassword,
  onCode,
}: {
  totp: boolean;
  password: string;
  code: string;
  onPassword: (password: string) => void;
  onCode: (code: string) => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <PasswordField
        toggleLabel="password"
        label={t("settings.sync.password")}
        autoComplete="current-password"
        value={password}
        onChange={(event) => {
          onPassword(event.target.value);
        }}
      />
      {totp && (
        <TextField
          label={t("settings.sync.code")}
          inputMode="numeric"
          autoComplete="one-time-code"
          mono
          value={code}
          onChange={(event) => {
            onCode(event.target.value);
          }}
        />
      )}
    </>
  );
}

function Options({
  email,
  accountId,
  totp,
  onGranted,
  onCancel,
}: {
  email: string;
  accountId: AccountId;
  totp: boolean;
  onGranted: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [all, setAll] = useState(true);
  const [chosen, setChosen] = useState<ReadonlySet<string>>(new Set());
  const [days, setDays] = useState<number>(defaultWaitDays);
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const scope: EmergencyScopeView = all ? { kind: "all" } : { kind: "vaults", vaultIds: [...chosen] };

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const result = await ipc.call("emergency_grant", {
      accountId,
      scope,
      waitDays: days,
      password,
      code: totp && code !== "" ? code : null,
    });
    setBusy(false);
    setPassword("");
    setCode("");
    if (result.ok) {
      onGranted();
    } else {
      setError(result.error);
    }
  }

  return (
    <form className="stack" noValidate onSubmit={(event) => void submit(event)}>
      <StepHeading>{t("emergency.grant.optionsTitle", { email: isolate(email) })}</StepHeading>
      <p className="body" role="status">
        {t("sharing.code.verified", { email: isolate(email) })}
      </p>
      <ScopeFields all={all} chosen={chosen} onAll={setAll} onChosen={setChosen} />
      <Select
        label={t("emergency.grant.wait")}
        value={String(days)}
        options={waitDays.map((each) => ({
          value: String(each),
          label: t("emergency.grant.wait.option", { days: each }),
        }))}
        onChange={(event) => {
          setDays(Number(event.target.value));
        }}
      />
      <p className="field__hint">{t("emergency.grant.waitHint")}</p>
      <Callout tone="warning">
        <p className="body">{t("emergency.grant.serverTrust")}</p>
        <p className="body">{t("emergency.grant.vaultKeysOnly")}</p>
      </Callout>
      <CredentialFields totp={totp} password={password} code={code} onPassword={setPassword} onCode={setCode} />
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onCancel}>{t("action.cancel")}</Button>
        <HoloButton type="submit" disabled={busy || password === "" || (!all && chosen.size === 0)}>
          {t("emergency.grant.submit")}
        </HoloButton>
      </div>
    </form>
  );
}

export function GrantDialog({
  totp,
  onGranted,
  onClose,
}: {
  totp: boolean;
  onGranted: (email: string) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [stage, setStage] = useState<Stage>({ kind: "email" });
  const [email, setEmail] = useState("");
  return (
    <Dialog title={t("emergency.grant.title")} onClose={onClose}>
      {stage.kind === "email" && (
        <form
          className="stack"
          noValidate
          onSubmit={(event) => {
            event.preventDefault();
            if (email.trim() !== "") {
              setStage({ kind: "verify", email: email.trim() });
            }
          }}
        >
          <TextField
            label={t("emergency.grant.lookup")}
            type="email"
            autoComplete="off"
            spellCheck={false}
            data-autofocus=""
            autoFocus
            value={email}
            onChange={(event) => {
              setEmail(event.target.value);
            }}
          />
          <div className="dialog__actions">
            <Button onClick={onClose}>{t("action.cancel")}</Button>
            <HoloButton type="submit" disabled={email.trim() === ""}>
              {t("sharing.lookup.find")}
            </HoloButton>
          </div>
        </form>
      )}
      {stage.kind === "verify" && (
        <ContactVerifier
          email={stage.email}
          onVerified={(accountId, verifiedEmail) => {
            setStage({ kind: "options", accountId, email: verifiedEmail });
          }}
          onCancel={() => {
            setStage({ kind: "email" });
          }}
        />
      )}
      {stage.kind === "options" && (
        <Options
          email={stage.email}
          accountId={stage.accountId}
          totp={totp}
          onGranted={() => {
            onGranted(stage.email);
            onClose();
          }}
          onCancel={onClose}
        />
      )}
    </Dialog>
  );
}
