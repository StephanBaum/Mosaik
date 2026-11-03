/**
 * Hardware-protected vaults (D21): the closed panel that asks for the security key, and the
 * bar above an open vault (countdown, "Lock this vault", keys, backup-key card). A closed
 * vault shows no item names: they are encrypted under the vault key (CRYPTO_SPEC §7.4).
 */
import { useEffect, useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { HardwareKeySummary, IpcError, IpcResult, VaultSummary } from "../../ipc/contract/index.ts";
import { AddKeyDialog, KeysDialog } from "../SecurityKeys.tsx";

/** The closed state: open the vault with the security key (and its PIN). */
export function HardwareVaultPanel({
  vault,
  onOpened,
}: {
  vault: VaultSummary;
  onOpened: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [pin, setPin] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    const submitted = pin === "" ? null : pin;
    setPin("");
    setBusy(true);
    setError(null);
    const result = await ipc.call("open_hardware_vault", { vaultId: vault.vaultId, pin: submitted });
    setBusy(false);
    if (result.ok) {
      onOpened();
    } else {
      setError(result.error);
    }
  }

  return (
    <section className="hw-panel" aria-labelledby="hw-panel-title">
      <form className="hw-panel__inner" onSubmit={(event) => void submit(event)} noValidate>
        <span className="hw-panel__icon" aria-hidden="true">
          <Icon name="securityKey" />
        </span>
        <h1 id="hw-panel-title" className="display">
          {t("vault.hardware.title")}
        </h1>
        <p className="lede">{t("vault.hardware.body")}</p>
        <PasswordField
          label={t("lock.quick.pin")}
          hint={t("vault.hardware.pinBody")}
          autoComplete="off"
          value={pin}
          onChange={(event) => {
            setPin(event.target.value);
          }}
        />
        <ErrorAlert error={error} />
        <p className="status" role="status">
          {busy ? t("lock.quick.touchKey") : ""}
        </p>
        <div className="actions hw-panel__actions">
          <HoloButton type="submit" ambient disabled={busy}>
            <Icon name="securityKey" />
            {t("vault.hardware.open")}
          </HoloButton>
          {busy && (
            <Button
              onClick={() => {
                void ipc.call("hardware_key_cancel", null);
              }}
            >
              {t("lock.quick.cancel")}
            </Button>
          )}
        </div>
        <p className="caption">
          <Icon name="lock" size="sm" /> {t("vault.hardware.lockedNote")}
        </p>
      </form>
    </section>
  );
}

function formatClock(milliseconds: number): string {
  const total = Math.max(Math.ceil(milliseconds / 1000), 0);
  return `${String(Math.floor(total / 60))}:${String(total % 60).padStart(2, "0")}`;
}

/** The time left until `openUntil`, as m:ss, updated every second; display only. */
function useCountdown(openUntil: number | null): string | null {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (openUntil === null) {
      return undefined;
    }
    const tick = setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => {
      clearInterval(tick);
    };
  }, [openUntil]);
  return openUntil === null ? null : formatClock(openUntil - now);
}

/** The keys of an open hardware vault, with the add and remove calls that keep them current. */
function useVaultKeys(vaultId: string) {
  const ipc = useIpc();
  const [keys, setKeys] = useState<readonly HardwareKeySummary[] | null>(null);
  useEffect(() => {
    let current = true;
    void ipc.call("hardware_vault_keys", { vaultId }).then((result) => {
      if (current && result.ok) {
        setKeys(result.value.keys);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, vaultId]);

  function settle(result: IpcResult<{ keys: HardwareKeySummary[] }>): IpcError | null {
    if (result.ok) {
      setKeys(result.value.keys);
    }
    return result.ok ? null : result.error;
  }
  return {
    keys,
    add: async (_password: string | null, pin: string | null) =>
      settle(await ipc.call("add_hardware_vault_key", { vaultId, pin })),
    remove: async (methodId: string) =>
      settle(await ipc.call("remove_hardware_vault_key", { vaultId, methodId })),
  };
}

function BackupCard({ onAdd }: { onAdd: () => void }) {
  const { t } = useI18n();
  return (
    <section className="hw-backup" aria-labelledby="hw-backup-title">
      <h2 id="hw-backup-title" className="subtitle">
        {t("vault.hardware.backupTitle")}
      </h2>
      <p className="caption">{t("vault.hardware.backupBody")}</p>
      <Button onClick={onAdd}>{t("settings.quick.addKey")}</Button>
    </section>
  );
}

/** The bar above an open hardware vault: countdown, lock, keys and the backup-key card. */
export function HardwareVaultBar({
  vault,
  onClosed,
}: {
  vault: VaultSummary;
  /** The user locked the vault (the event covers the timer and the session lock). */
  onClosed: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { keys, add, remove } = useVaultKeys(vault.vaultId);
  const [dialog, setDialog] = useState<"keys" | "backup" | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const left = useCountdown(vault.openUntil);
  const close = () => {
    setDialog(null);
  };

  async function lockNow() {
    const result = await ipc.call("close_hardware_vault", { vaultId: vault.vaultId });
    if (result.ok) {
      onClosed();
    } else {
      setError(result.error);
    }
  }

  return (
    <div className="hw-bar-wrap">
      <div className="hw-bar">
        <p className="hw-bar__title">
          <Icon name="lockOpen" />
          <span>{t("vault.hardware.codesOnly")}</span>
          {left !== null && (
            <span className="hw-bar__time tabular">{t("vault.hardware.closesIn", { time: left })}</span>
          )}
        </p>
        <Button
          onClick={() => {
            setDialog("keys");
          }}
        >
          {t("vault.hardware.keys")}
        </Button>
        <Button
          onClick={() => {
            void lockNow();
          }}
        >
          <Icon name="lock" />
          {t("vault.hardware.lockNow")}
        </Button>
      </div>
      <ErrorAlert error={error} />
      {keys?.length === 1 && (
        <BackupCard
          onAdd={() => {
            setDialog("backup");
          }}
        />
      )}
      {dialog === "backup" && <AddKeyDialog withPassword={false} onAdd={add} onClose={close} />}
      {dialog === "keys" && keys !== null && (
        <KeysDialog
          title={t("vault.hardware.keysTitle")}
          keys={keys}
          note={t("vault.hardware.removeNote")}
          withPassword={false}
          onAdd={add}
          onRemove={remove}
          onClose={close}
        />
      )}
    </div>
  );
}
