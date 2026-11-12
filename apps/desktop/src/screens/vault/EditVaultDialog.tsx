/**
 * "Edit vault": the name of a vault (`rename_vault`). Opened from the "Vault details were
 * shortened" notice so the person can type the value again.
 */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, VaultSummary } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";

export function EditVaultDialog({
  vault,
  onSaved,
  onClose,
  returnFocus,
}: {
  vault: VaultSummary;
  onSaved: () => void;
  onClose: () => void;
  returnFocus?: () => HTMLElement | null;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [name, setName] = useState(vault.name ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const result = await ipc.call("rename_vault", { vaultId: vault.vaultId, name });
    setBusy(false);
    if (result.ok) {
      onSaved();
      onClose();
    } else {
      setError(result.error);
    }
  }

  return (
    <Dialog title={t("vault.edit.title")} onClose={onClose} {...(returnFocus === undefined ? {} : { returnFocus })}>
      <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
        <TextField
          label={t("vault.create.name")}
          data-autofocus=""
          autoComplete="off"
          value={name}
          onChange={(event) => {
            setName(event.target.value);
          }}
        />
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button onClick={onClose}>{t("action.cancel")}</Button>
          <HoloButton type="submit" disabled={busy || name.trim() === ""}>
            {t("action.save")}
          </HoloButton>
        </div>
      </form>
    </Dialog>
  );
}
