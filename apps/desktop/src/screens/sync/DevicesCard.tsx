/**
 * The devices of the sync account: this device marked, Rename, and Remove (Mosaic server
 * only: it signs the device out but takes nothing from its copy). A folder has no way to lock
 * a device out, so it says what to do instead.
 */
import { useEffect, useRef, useState, type KeyboardEvent, type Ref, type SyntheticEvent } from "react";
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
import { SetupPhone } from "./SetupPhone.tsx";

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

  // Esc cancels the rename (and not anything behind it).
  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      onCancel();
    }
  }

  return (
    <li className="row">
      <form
        className="row__main stack"
        onSubmit={(event) => void save(event)}
        onKeyDown={onKeyDown}
        noValidate
      >
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
  canRemove,
  onEdit,
  onRemove,
}: {
  device: SyncDevice;
  now: number;
  /** A folder can rename this device only; a signed-out device renames nothing. */
  canRename: boolean;
  canRemove: boolean;
  onEdit: () => void;
  onRemove: () => void;
}) {
  const { t, locale } = useI18n();
  const label = device.name ?? t("settings.sync.devices.unnamed");
  return (
    <li className="row">
      <div className="row__main">
        <p className="row__value">
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
          data-rename-for={device.deviceId}
          variant="ghost"
          size="sm"
          aria-label={t("settings.sync.devices.renameNamed", { name: label })}
          onClick={onEdit}
        >
          {t("settings.sync.devices.rename")}
        </Button>
      )}
      {canRemove && device.canRemove && (
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

/**
 * Where focus goes when the control that had it is gone: the Rename button after an edit ended
 * (saved, cancelled or Esc), the Devices heading after a device was removed.
 */
function useDeviceFocus(devices: readonly SyncDevice[], editing: string | null) {
  const heading = useRef<HTMLHeadingElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const afterEdit = useRef<string | null>(null);
  const afterRemove = useRef(false);
  useEffect(() => {
    if (editing === null && afterEdit.current !== null) {
      const id = afterEdit.current;
      afterEdit.current = null;
      Array.from(list.current?.querySelectorAll<HTMLElement>("[data-rename-for]") ?? [])
        .find((button) => button.dataset["renameFor"] === id)
        ?.focus();
    }
  }, [editing]);
  useEffect(() => {
    if (afterRemove.current) {
      afterRemove.current = false;
      heading.current?.focus();
    }
  }, [devices]);
  return {
    heading,
    list,
    /** The edit of this device ends: its Rename button gets focus when it is back. */
    editEnded: (deviceId: string) => {
      afterEdit.current = deviceId;
    },
    /** A device was removed: the list reloads, then the heading gets focus. */
    removed: () => {
      afterRemove.current = true;
    },
  };
}

function DeviceList({
  devices,
  config,
  signedOut,
  editing,
  listRef,
  onRename,
  onEdit,
  onStopEditing,
  onRemove,
}: {
  devices: readonly SyncDevice[];
  config: SyncConfigView;
  signedOut: boolean;
  editing: string | null;
  listRef: Ref<HTMLUListElement>;
  onRename: (device: SyncDevice, name: string) => Promise<IpcError | null>;
  onEdit: (deviceId: string) => void;
  onStopEditing: (deviceId: string) => void;
  onRemove: (device: SyncDevice) => void;
}) {
  const now = useNow();
  return (
    <ul className="rows" ref={listRef}>
      {devices.map((device) =>
        editing === device.deviceId && !signedOut ? (
          <RenameRow
            key={device.deviceId}
            device={device}
            onRename={(name) => onRename(device, name)}
            onCancel={() => {
              onStopEditing(device.deviceId);
            }}
          />
        ) : (
          <DeviceRow
            key={device.deviceId}
            device={device}
            now={now}
            canRename={!signedOut && (device.current || config.kind === "hosted")}
            canRemove={!signedOut}
            onEdit={() => {
              onEdit(device.deviceId);
            }}
            onRemove={() => {
              onRemove(device);
            }}
          />
        ),
      )}
    </ul>
  );
}

export function DevicesCard({
  config,
  signedOut,
  refreshKey,
  onRenamed,
}: {
  config: SyncConfigView;
  /** The server ended this device's session: renaming and removing wait for the sign-in. */
  signedOut: boolean;
  /** Changes when a sync run finished, to read the list again. */
  refreshKey: number | null;
  /** This device's own name changed. */
  onRenamed: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { devices, error, reload } = useDevices(refreshKey);
  const [editing, setEditing] = useState<string | null>(null);
  const [removing, setRemoving] = useState<SyncDevice | null>(null);
  const { heading, list, editEnded, removed } = useDeviceFocus(devices, editing);

  function stopEditing(deviceId: string) {
    editEnded(deviceId);
    setEditing(null);
  }

  async function rename(device: SyncDevice, name: string): Promise<IpcError | null> {
    const result = await ipc.call("sync_device_rename", { deviceId: device.deviceId, name });
    if (!result.ok) {
      return result.error;
    }
    stopEditing(device.deviceId);
    reload();
    if (device.current) {
      onRenamed();
    }
    return null;
  }

  return (
    <div className="stack">
      <h3 className="subtitle" ref={heading} tabIndex={-1}>
        {t("settings.sync.devices.title")}
      </h3>
      <ErrorAlert error={error} />
      <DeviceList
        devices={devices}
        config={config}
        signedOut={signedOut}
        editing={editing}
        listRef={list}
        onRename={rename}
        onEdit={setEditing}
        onStopEditing={stopEditing}
        onRemove={setRemoving}
      />
      {config.kind === "folder" && (
        <p className="field__hint">{t("settings.sync.devices.folderNote")}</p>
      )}
      <SetupPhone config={config} />
      {removing !== null && (
        <RemoveDialog
          device={removing}
          needsCode={config.kind === "hosted" && config.totpEnabled}
          onRemoved={() => {
            removed();
            reload();
          }}
          onClose={() => {
            setRemoving(null);
          }}
        />
      )}
    </div>
  );
}
