/**
 * The devices of the sync account: this device marked, Rename, and Remove (Mosaic server
 * only: it signs the device out but takes nothing from its copy). A folder has no way to lock
 * a device out, so it says what to do instead.
 */
import { useEffect, useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { Chip } from "../../components/Chip.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { TextField } from "../../components/TextField.tsx";
import { UserText } from "../../components/UserText.tsx";
import { formatRelative } from "../../i18n/format.ts";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, SyncConfigView, SyncDevice } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { useNow } from "./hooks.ts";
import { PasswordDialog } from "./PasswordDialog.tsx";

function RenameRow({
  device,
  onRename,
  onCancel,
}: {
  device: SyncDevice;
  onRename: (name: string) => Promise<IpcError | null>;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [name, setName] = useState(device.name ?? "");
  const [error, setError] = useState<IpcError | null>(null);

  async function save(event: SyntheticEvent) {
    event.preventDefault();
    setError(await onRename(name.trim()));
  }

  return (
    <li className="row">
      <form className="row__main stack" onSubmit={(event) => void save(event)} noValidate>
        <TextField
          label={t("settings.sync.devices.renameLabel")}
          autoComplete="off"
          autoFocus
          value={name}
          onChange={(event) => {
            setName(event.target.value);
          }}
        />
        <ErrorAlert error={error} />
        <div className="actions">
          <Button type="submit" size="sm" disabled={name.trim() === ""}>
            {t("action.save")}
          </Button>
          <Button variant="ghost" size="sm" onClick={onCancel}>
            {t("action.cancel")}
          </Button>
        </div>
      </form>
    </li>
  );
}

function DeviceRow({
  device,
  now,
  canRename,
  onEdit,
  onRemove,
}: {
  device: SyncDevice;
  now: number;
  /** A folder can rename this device only. */
  canRename: boolean;
  onEdit: () => void;
  onRemove: () => void;
}) {
  const { t, locale } = useI18n();
  const label = device.name ?? t("settings.sync.devices.unnamed");
  return (
    <li className="row">
      <div className="row__main">
        <p>
          {device.name === null ? label : <UserText>{device.name}</UserText>}{" "}
          {device.current && <Chip>{t("settings.sync.devices.thisDevice")}</Chip>}
        </p>
        <p className="caption">
          {device.lastSeenAt === null
            ? t("settings.sync.devices.neverSeen")
            : t("settings.sync.devices.lastSeen", {
                when: formatRelative(locale, device.lastSeenAt, Math.max(now, device.lastSeenAt)),
              })}
        </p>
      </div>
      {canRename && (
        <Button
          variant="ghost"
          size="sm"
          aria-label={t("settings.sync.devices.renameNamed", { name: label })}
          onClick={onEdit}
        >
          {t("settings.sync.devices.rename")}
        </Button>
      )}
      {device.canRemove && (
        <Button
          variant="ghost"
          size="sm"
          aria-label={t("settings.sync.devices.removeNamed", { name: label })}
          onClick={onRemove}
        >
          {t("settings.sync.devices.remove")}
        </Button>
      )}
    </li>
  );
}

/** The device list, read again when a sync run finished or after a change. */
function useDevices(refreshKey: number | null) {
  const ipc = useIpc();
  const [devices, setDevices] = useState<SyncDevice[]>([]);
  const [error, setError] = useState<IpcError | null>(null);
  const [version, setVersion] = useState(0);
  useEffect(() => {
    let current = true;
    void ipc.call("sync_devices", null).then((result) => {
      if (!current) {
        return;
      }
      if (result.ok) {
        setDevices(result.value.devices);
        setError(null);
      } else {
        setError(result.error);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, refreshKey, version]);
  return { devices, error, reload: () => { setVersion((n) => n + 1); } };
}

function RemoveDialog({
  device,
  needsCode,
  onRemoved,
  onClose,
}: {
  device: SyncDevice;
  needsCode: boolean;
  onRemoved: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  return (
    <PasswordDialog
      title={t("settings.sync.devices.removeTitle", {
        name: device.name ?? t("settings.sync.devices.unnamed"),
      })}
      body={t("settings.sync.devices.removeBody")}
      withCode={needsCode}
      submitLabel={t("settings.sync.devices.removeConfirm")}
      submitVariant="dangerSolid"
      onSubmit={async (password, code) => {
        const result = await ipc.call("sync_device_remove", {
          deviceId: device.deviceId,
          password,
          code,
        });
        if (result.ok) {
          onRemoved();
        }
        return result.ok ? null : result.error;
      }}
      onClose={onClose}
    />
  );
}

export function DevicesCard({
  config,
  refreshKey,
  onRenamed,
}: {
  config: SyncConfigView;
  /** Changes when a sync run finished, to read the list again. */
  refreshKey: number | null;
  /** This device's own name changed. */
  onRenamed: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const now = useNow();
  const { devices, error, reload } = useDevices(refreshKey);
  const [editing, setEditing] = useState<string | null>(null);
  const [removing, setRemoving] = useState<SyncDevice | null>(null);

  async function rename(device: SyncDevice, name: string): Promise<IpcError | null> {
    const result = await ipc.call("sync_device_rename", { deviceId: device.deviceId, name });
    if (!result.ok) {
      return result.error;
    }
    setEditing(null);
    reload();
    if (device.current) {
      onRenamed();
    }
    return null;
  }

  return (
    <div className="stack">
      <h3 className="subtitle">{t("settings.sync.devices.title")}</h3>
      <ErrorAlert error={error} />
      <ul className="rows">
        {devices.map((device) =>
          editing === device.deviceId ? (
            <RenameRow
              key={device.deviceId}
              device={device}
              onRename={(name) => rename(device, name)}
              onCancel={() => {
                setEditing(null);
              }}
            />
          ) : (
            <DeviceRow
              key={device.deviceId}
              device={device}
              now={now}
              canRename={device.current || config.kind === "hosted"}
              onEdit={() => {
                setEditing(device.deviceId);
              }}
              onRemove={() => {
                setRemoving(device);
              }}
            />
          ),
        )}
      </ul>
      {config.kind === "folder" && (
        <p className="field__hint">{t("settings.sync.devices.folderNote")}</p>
      )}
      {removing !== null && (
        <RemoveDialog
          device={removing}
          needsCode={config.kind === "hosted" && config.totpEnabled}
          onRemoved={reload}
          onClose={() => {
            setRemoving(null);
          }}
        />
      )}
    </div>
  );
}
