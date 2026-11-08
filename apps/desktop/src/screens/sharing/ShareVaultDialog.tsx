/**
 * Share a vault: an email address, the safety-code comparison (required, CRYPTO_SPEC §8.2), a
 * role, and the honesty line about what sharing cannot take back. Mosaic seals the vault key to
 * the person's key only after all of that.
 */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { TextField } from "../../components/TextField.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { AccountId, IpcError, ShareRole, VaultSummary } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "../sync/hooks.ts";
import { vaultLabel } from "../vault/Sidebar.tsx";
import { ContactVerifier } from "./ContactVerifier.tsx";

type Stage =
  | { kind: "email" }
  | { kind: "verify"; email: string }
  | { kind: "role"; accountId: AccountId; email: string }
  | { kind: "sent"; email: string };

const GRANTABLE: readonly Exclude<ShareRole, "owner">[] = ["read", "write", "manage"];

function EmailStep({ onNext, onCancel }: { onNext: (email: string) => void; onCancel: () => void }) {
  const { t } = useI18n();
  const [email, setEmail] = useState("");
  return (
    <form
      className="stack"
      noValidate
      onSubmit={(event: SyntheticEvent) => {
        event.preventDefault();
        if (email.trim() !== "") {
          onNext(email.trim());
        }
      }}
    >
      <TextField
        label={t("sharing.lookup.email")}
        type="email"
        autoComplete="off"
        spellCheck={false}
        data-autofocus=""
        value={email}
        onChange={(event) => {
          setEmail(event.target.value);
        }}
      />
      <div className="dialog__actions">
        <Button onClick={onCancel}>{t("action.cancel")}</Button>
        <HoloButton type="submit" disabled={email.trim() === ""}>
          {t("sharing.lookup.find")}
        </HoloButton>
      </div>
    </form>
  );
}

function RoleStep({
  vault,
  accountId,
  email,
  onSent,
  onCancel,
}: {
  vault: VaultSummary;
  accountId: AccountId;
  email: string;
  onSent: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [role, setRole] = useState<Exclude<ShareRole, "owner">>("read");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function send(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const result = await ipc.call("vault_share", { vaultId: vault.vaultId, accountId, role });
    setBusy(false);
    if (result.ok) {
      onSent();
    } else {
      setError(result.error);
    }
  }

  return (
    <form className="stack" noValidate onSubmit={(event) => void send(event)}>
      <p className="body" role="status">
        {t("sharing.code.verified", { email: isolate(email) })}
      </p>
      <fieldset className="fieldset">
        <legend className="fieldset__legend">{t("sharing.role.label")}</legend>
        {GRANTABLE.map((each) => (
          <label key={each} className="check">
            <input
              type="radio"
              name="share-role"
              checked={role === each}
              onChange={() => {
                setRole(each);
              }}
            />
            <span>
              {t(`sharing.role.${each}`)}
              <span className="field__hint">{t(`sharing.role.desc.${each}`)}</span>
            </span>
          </label>
        ))}
      </fieldset>
      <Callout tone="info">
        <p className="body">{t("sharing.role.honesty")}</p>
      </Callout>
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onCancel}>{t("action.cancel")}</Button>
        <HoloButton type="submit" disabled={busy}>
          {t("sharing.invite.send")}
        </HoloButton>
      </div>
    </form>
  );
}

export function ShareVaultDialog({
  vault,
  onShared,
  onClose,
}: {
  vault: VaultSummary;
  /** An invite was sent (the members list reads again). */
  onShared: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [stage, setStage] = useState<Stage>({ kind: "email" });
  const name = vaultLabel(vault, (key) => t(key));
  return (
    <Dialog title={t("sharing.share.title", { vault: isolate(name) })} onClose={onClose}>
      <div className="stack">
        <p className="caption">
          <UserText>{name}</UserText>
        </p>
        {stage.kind === "email" && (
          <EmailStep
            onNext={(email) => {
              setStage({ kind: "verify", email });
            }}
            onCancel={onClose}
          />
        )}
        {stage.kind === "verify" && (
          <ContactVerifier
            email={stage.email}
            onVerified={(accountId, email) => {
              setStage({ kind: "role", accountId, email });
            }}
            onCancel={() => {
              setStage({ kind: "email" });
            }}
          />
        )}
        {stage.kind === "role" && (
          <RoleStep
            vault={vault}
            accountId={stage.accountId}
            email={stage.email}
            onSent={() => {
              onShared();
              setStage({ kind: "sent", email: stage.email });
            }}
            onCancel={onClose}
          />
        )}
        {stage.kind === "sent" && (
          <>
            <p className="body" role="status">
              {t("sharing.invite.sent", { email: isolate(stage.email) })}
            </p>
            <div className="dialog__actions">
              <HoloButton onClick={onClose}>{t("action.done")}</HoloButton>
            </div>
          </>
        )}
      </div>
    </Dialog>
  );
}
