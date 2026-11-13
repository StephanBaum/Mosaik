/**
 * Hardware-protected vaults (D21): the closed panel that asks for the security key, and the
 * bar above an open vault (countdown, "Lock this vault", keys, backup-key card). A closed
 * vault shows no item names: they are encrypted under the vault key (CRYPTO_SPEC §7.4).
 */
import { useEffect, useRef, useState, type RefObject, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useReauth } from "../../hooks/useReauth.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { HardwareKeySummary, IpcError, IpcResult, VaultSummary } from "../../ipc/contract/index.ts";
import { AddKeyDialog, KeysDialog } from "../SecurityKeys.tsx";

/** Opening and closing a hardware vault move focus and announce it (`ListView` tells the panel). */
export type VaultTransition = { kind: "opened" | "closed"; vaultId: string };

/** The closed panel's state: PIN, waiting for the key, the error, and the "locked itself" notice. */
function useOpenVault(
  vault: VaultSummary,
  form: RefObject<HTMLFormElement | null>,
  events: { justClosed: boolean; onOpened: () => void; onSeen: (() => void) | undefined },
) {
  const { justClosed, onOpened, onSeen } = events;
  const { t } = useI18n();
  const ipc = useIpc();
  const [pin, setPin] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const [notice, setNotice] = useState("");
  const announce = useRef(justClosed);

  useEffect(() => {
    // The live region is already on screen when the text arrives, so it is read out.
    if (announce.current) {
      announce.current = false;
      setNotice(t("vault.hardware.selfLocked"));
      onSeen?.();
    }
  }, [t, onSeen]);

  // While the key is waited for, the focused button is disabled: move to Cancel.
  useEffect(() => {
    if (busy) {
      form.current?.querySelector<HTMLElement>("[data-cancel]")?.focus();
    }
  }, [busy, form]);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    const submitted = pin === "" ? null : pin;
    setPin("");
    setBusy(true);
    setError(null);
    setNotice("");
    const result = await ipc.call("open_hardware_vault", { vaultId: vault.vaultId, pin: submitted });
    setBusy(false);
    if (result.ok) {
      onOpened();
      return;
    }
    // A deliberate cancel says nothing. Either way, back to where the user acted.
    const cancelled = result.error.code === "cancelled";
    setError(cancelled ? null : result.error);
    requestAnimationFrame(() => {
      form.current?.querySelector<HTMLElement>(cancelled ? "button[type=submit]" : "input")?.focus();
    });
  }
  return { pin, setPin, busy, error, notice, submit };
}

/** The closed state: open the vault with the security key (and its PIN). */
export function HardwareVaultPanel({
  vault,
  justClosed = false,
  onOpened,
  onSeen,
}: {
  vault: VaultSummary;
  /** The vault was open a moment ago and locked itself or was locked by the user. */
  justClosed?: boolean;
  onOpened: () => void;
  /** The panel has announced the close; forget it. */
  onSeen?: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const form = useRef<HTMLFormElement>(null);
  const { pin, setPin, busy, error, notice, submit } = useOpenVault(vault, form, {
    justClosed,
    onOpened,
    onSeen,
  });

  return (
    <section className="hw-panel" aria-labelledby="hw-panel-title">
      <form ref={form} className="hw-panel__inner" onSubmit={(event) => void submit(event)} noValidate>
        <span className="hw-panel__icon" aria-hidden="true">
          <Icon name="securityKey" />
        </span>
        <div className="hw-panel__head">
          <p className="caption">
            {t("vault.hardware.placeholderName")} · {t("vault.hardware.eyebrow")}
          </p>
          <h1 id="hw-panel-title" className="display">
            {t("vault.hardware.title")}
          </h1>
          <p className="lede">{t("vault.hardware.body")}</p>
        </div>
        <PasswordField
          label={t("lock.quick.pin")}
          hint={t("vault.hardware.pinBody")}
          autoComplete="off"
          autoFocus={justClosed}
          value={pin}
          onChange={(event) => {
            setPin(event.target.value);
          }}
        />
        <ErrorAlert error={error} />
        <p className="status" role="status">
          {busy ? t("lock.quick.touchKey") : notice}
        </p>
        <div className="actions">
          <HoloButton type="submit" ambient disabled={busy}>
            <Icon name="securityKey" />
            {t("vault.hardware.open")}
          </HoloButton>
          {busy && (
            <Button
              data-cancel=""
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

/** The warning before the vault locks itself (WCAG 2.2.1: tell the user, then give a way on). */
const WARN_MS = 30_000;

/** The milliseconds left until `openUntil`, updated every second; display only. */
function useTimeLeft(openUntil: number | null): number | null {
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
  return openUntil === null ? null : openUntil - now;
}

/** The keys of an open hardware vault, with the add and remove calls that keep them current. */
function useVaultKeys(vaultId: string) {
  const ipc = useIpc();
  const { guard, dialog } = useReauth("reauth.bodyKey", "hardwareVaultKey");
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
    // Adding a key needs a fresh master password: ask for it when the backend says so.
    add: async (_password: string | null, pin: string | null) =>
      settle(await guard(() => ipc.call("add_hardware_vault_key", { vaultId, pin }))),
    remove: async (methodId: string) =>
      settle(await ipc.call("remove_hardware_vault_key", { vaultId, methodId })),
    reauthDialog: dialog,
  };
}

function BackupCard({ onAdd }: { onAdd: () => void }) {
  const { t } = useI18n();
  return (
    <div className="hw-backup-wrap">
      <Callout tone="warning">
        <section className="hw-backup" aria-labelledby="hw-backup-title">
          <div className="hw-backup__text">
            <h2 id="hw-backup-title" className="hw-backup__title">
              {t("vault.hardware.backupTitle")}
            </h2>
            <p className="caption">{t("vault.hardware.backupBody")}</p>
          </div>
          <Button onClick={onAdd}>{t("settings.quick.addKey")}</Button>
        </section>
      </Callout>
    </div>
  );
}

function BarHeader({
  left,
  focusLock,
  onKeys,
  onLock,
}: {
  left: number | null;
  focusLock: boolean;
  onKeys: () => void;
  onLock: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="hw-bar">
      <p className="hw-bar__title">
        <Icon name="lockOpen" />
        <span>{t("vault.hardware.codesOnly")}</span>
        {left !== null && (
          <span className="hw-bar__time tabular">
            {t("vault.hardware.closesIn", { time: formatClock(left) })}
          </span>
        )}
      </p>
      <Button aria-label={t("vault.hardware.keysLabel")} onClick={onKeys}>
        {t("vault.hardware.keys")}
      </Button>
      <Button autoFocus={focusLock} onClick={onLock}>
        <Icon name="lock" />
        {t("vault.hardware.lockNow")}
      </Button>
    </div>
  );
}

/** Always on screen so that a screen reader announces the text when it appears. */
function ClosingWarning({ show }: { show: boolean }) {
  const { t } = useI18n();
  return (
    <p className="hw-bar__warning" role="status">
      {show && (
        <>
          <Icon name="alert" size="sm" />
          {t("vault.hardware.closingSoon")}
        </>
      )}
    </p>
  );
}

/** The bar above an open hardware vault: countdown, lock, keys and the backup-key card. */
export function HardwareVaultBar({
  vault,
  focusLock = false,
  onClosed,
  onSeen,
}: {
  vault: VaultSummary;
  /** The vault was opened a moment ago: put focus on "Lock this vault". */
  focusLock?: boolean;
  /** The user locked the vault (the event covers the timer and the session lock). */
  onClosed: () => void;
  onSeen?: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { keys, add, remove, reauthDialog } = useVaultKeys(vault.vaultId);
  const [dialog, setDialog] = useState<"keys" | "backup" | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const left = useTimeLeft(vault.openUntil);
  const close = () => {
    setDialog(null);
  };

  useEffect(() => {
    if (focusLock) {
      onSeen?.();
    }
  }, [focusLock, onSeen]);

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
      <BarHeader
        left={left}
        focusLock={focusLock}
        onKeys={() => {
          setDialog("keys");
        }}
        onLock={() => {
          void lockNow();
        }}
      />
      <ClosingWarning show={left !== null && left > 0 && left <= WARN_MS} />
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
      {reauthDialog}
    </div>
  );
}
