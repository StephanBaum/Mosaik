/**
 * New vault. With security-key support the dialog offers a hardware-protected vault for
 * one-time codes (CRYPTO_SPEC §7.4) and spells out the trade-off before the user commits.
 */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { Switch } from "../../components/Switch.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { IpcError, VaultSummary } from "../../ipc/contract/index.ts";
import { useHardwareKeysSupported } from "./useVaultData.ts";

function HardwareOptions({
  hardware,
  pin,
  onHardware,
  onPin,
}: {
  hardware: boolean;
  pin: string;
  onHardware: (on: boolean) => void;
  onPin: (pin: string) => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <Switch
        label={t("vault.hardware.createOption")}
        checked={hardware}
        onChange={(event) => {
          onHardware(event.target.checked);
        }}
      />
      {hardware && (
        <>
          <Callout tone="warning">
            <p>{t("vault.hardware.tradeoff")}</p>
            <p>{t("vault.hardware.moveNote")}</p>
          </Callout>
          <PasswordField
            label={t("lock.quick.pin")}
            hint={t("vault.hardware.pinBody")}
            autoComplete="off"
            value={pin}
            onChange={(event) => {
              onPin(event.target.value);
            }}
          />
        </>
      )}
    </>
  );
}

export function CreateVaultDialog({
  onCreated,
  onClose,
}: {
  onCreated: (vault: VaultSummary) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const hardwareKeysSupported = useHardwareKeysSupported();
  const [name, setName] = useState("");
  const [hardware, setHardware] = useState(false);
  const [pin, setPin] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const submittedPin = pin === "" ? null : pin;
    setPin("");
    const result = hardware
      ? await ipc.call("create_hardware_vault", { name, pin: submittedPin })
      : await ipc.call("create_vault", { name });
    setBusy(false);
    if (result.ok) {
      onCreated(result.value);
      onClose();
    } else {
      setError(result.error);
    }
  }

  function cancel() {
    if (busy && hardware) {
      void ipc.call("hardware_key_cancel", null);
    }
    onClose();
  }

  return (
    <Dialog title={t("vault.create")} onClose={cancel}>
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
        {hardwareKeysSupported && (
          <HardwareOptions hardware={hardware} pin={pin} onHardware={setHardware} onPin={setPin} />
        )}
        <ErrorAlert error={error} />
        <p className="status" role="status">
          {busy && hardware ? t("lock.quick.touchKey") : ""}
        </p>
        <div className="dialog__actions">
          <Button onClick={cancel}>{t("action.cancel")}</Button>
          <HoloButton type="submit" disabled={busy || name.trim() === ""}>
            {t("vault.create.submit")}
          </HoloButton>
        </div>
      </form>
    </Dialog>
  );
}
