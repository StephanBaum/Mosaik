/** Sync is off: the two ways to set it up, a Mosaic server and your own cloud folder. */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { TextField } from "../../components/TextField.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type {
  FolderChoice,
  IpcError,
  ServerCheck,
  SyncOverview,
} from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { HostedForm } from "./HostedForm.tsx";
import { useDefaultDeviceName } from "./hooks.ts";

function ServerCard({ onConnected }: { onConnected: (overview: SyncOverview) => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [url, setUrl] = useState("");
  const [check, setCheck] = useState<ServerCheck | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setCheck(null);
    const result = await ipc.call("sync_check_server", { serverUrl: url.trim() });
    setBusy(false);
    if (result.ok) {
      setCheck(result.value);
    } else {
      setError(result.error);
    }
  }

  return (
    <div className="card">
      <h3 className="subtitle">{t("settings.sync.server.title")}</h3>
      <p className="caption">{t("settings.sync.server.body")}</p>
      <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
        <TextField
          label={t("settings.sync.server.url")}
          hint={t("settings.sync.server.urlHint")}
          type="url"
          autoComplete="url"
          value={url}
          onChange={(event) => {
            setUrl(event.target.value);
            setCheck(null);
          }}
        />
        <ErrorAlert error={error} />
        <p className="status caption" role="status">
          {busy
            ? t("settings.sync.server.checking")
            : check === null
              ? ""
              : t(`settings.sync.server.registration.${check.registration}`)}
        </p>
        <div className="actions">
          <Button type="submit" disabled={busy || url.trim() === ""}>
            {t("settings.sync.server.check")}
          </Button>
        </div>
      </form>
      {check !== null && (
        <HostedForm
          key={check.serverUrl}
          serverUrl={check.serverUrl}
          registration={check.registration}
          onConnected={onConnected}
        />
      )}
      <p className="field__hint">{t("settings.sync.server.sees")}</p>
    </div>
  );
}

/** What the chosen folder holds, and Connect (not for a folder with other files). */
function FolderDetails({
  choice,
  busy,
  onConnect,
}: {
  choice: FolderChoice;
  busy: boolean;
  onConnect: (deviceName: string) => void;
}) {
  const { t } = useI18n();
  const defaultName = useDefaultDeviceName();
  const [name, setName] = useState(defaultName);
  return (
    <div className="stack">
      <p className="body">
        <span className="muted">{t("settings.sync.folder.path")}: </span>
        <UserText>{choice.displayPath}</UserText>
      </p>
      {choice.contents === "other" ? (
        <ErrorAlert
          error={{ code: "syncFolderNotEmpty" }}
          message={t("settings.sync.folder.contents.other")}
        />
      ) : (
        <>
          <p className="field__hint">{t(`settings.sync.folder.contents.${choice.contents}`)}</p>
          <TextField
            label={t("settings.sync.deviceName")}
            autoComplete="off"
            value={name}
            onChange={(event) => {
              setName(event.target.value);
            }}
          />
          <p className="status caption" role="status">
            {busy ? t("settings.sync.folder.working") : ""}
          </p>
          <div className="actions">
            <Button
              disabled={busy}
              onClick={() => {
                onConnect(name.trim());
              }}
            >
              {t("settings.sync.folder.connect")}
            </Button>
          </div>
        </>
      )}
    </div>
  );
}

function FolderCard({ onConnected }: { onConnected: (overview: SyncOverview) => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [choice, setChoice] = useState<FolderChoice | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function choose() {
    setError(null);
    const result = await ipc.call("sync_pick_folder", null);
    if (!result.ok) {
      setError(result.error);
    } else if (result.value !== null) {
      // `null` is a cancelled dialog: keep what was chosen before.
      setChoice(result.value);
    }
  }

  async function connect(choiceId: string, deviceName: string) {
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_folder_connect", { choiceId, deviceName });
    setBusy(false);
    if (result.ok) {
      onConnected(result.value);
    } else {
      setError(result.error);
    }
  }

  return (
    <div className="card">
      <h3 className="subtitle">{t("settings.sync.folder.title")}</h3>
      <p className="caption">{t("settings.sync.folder.body")}</p>
      <div className="actions">
        <Button
          onClick={() => {
            void choose();
          }}
        >
          {t(choice === null ? "settings.sync.folder.choose" : "settings.sync.folder.chooseOther")}
        </Button>
      </div>
      {choice !== null && (
        <FolderDetails
          key={choice.choiceId}
          choice={choice}
          busy={busy}
          onConnect={(deviceName) => {
            void connect(choice.choiceId, deviceName);
          }}
        />
      )}
      <ErrorAlert error={error} />
      <p className="field__hint">{t("settings.sync.folder.sees")}</p>
    </div>
  );
}

export function SetupCards({ onConnected }: { onConnected: (overview: SyncOverview) => void }) {
  const { t } = useI18n();
  return (
    <div className="stack">
      <p className="field__hint">{t("settings.sync.notSetUp")}</p>
      <ServerCard onConnected={onConnected} />
      <FolderCard onConnected={onConnected} />
    </div>
  );
}
