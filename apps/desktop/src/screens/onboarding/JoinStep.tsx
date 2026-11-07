/**
 * "I already use Mosaic": add this device to an existing account (M6). The account's data comes
 * from a Mosaic server or an own cloud folder; the master password and the Secret Key from the
 * Emergency Kit open it. The joined account is already kit-verified, so onboarding ends here.
 */
import { useId, useState, type KeyboardEvent, type SyntheticEvent } from "react";
import { AuthShell } from "../../components/AuthShell.tsx";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { Select } from "../../components/Select.tsx";
import { TextField } from "../../components/TextField.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type {
  FolderChoice,
  IpcError,
  JoinOutcome,
  IpcResult,
  SessionState,
} from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { useDefaultDeviceName } from "../sync/hooks.ts";

type Tab = "server" | "folder";
const TABS: readonly Tab[] = ["server", "folder"];

function Tabs({ id, tab, onTab }: { id: string; tab: Tab; onTab: (tab: Tab) => void }) {
  const { t } = useI18n();
  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
      event.preventDefault();
      const next = tab === "server" ? "folder" : "server";
      onTab(next);
      document.getElementById(`${id}-${next}`)?.focus();
    }
  }
  return (
    <div className="segmented" role="tablist" aria-label={t("onboarding.join.tabs")} onKeyDown={onKeyDown}>
      {TABS.map((each) => (
        <button
          key={each}
          id={`${id}-${each}`}
          type="button"
          role="tab"
          className="segmented__tab"
          aria-selected={each === tab}
          aria-controls={`${id}-panel`}
          tabIndex={each === tab ? 0 : -1}
          onClick={() => {
            onTab(each);
          }}
        >
          {t(`onboarding.join.tab.${each}`)}
        </button>
      ))}
    </div>
  );
}

function CodeForm({
  busy,
  error,
  onSubmit,
  onBack,
}: {
  busy: boolean;
  error: IpcError | null;
  onSubmit: (code: string) => void;
  onBack: () => void;
}) {
  const { t } = useI18n();
  const [code, setCode] = useState("");
  return (
    <form
      className="form-stack"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit(code);
        setCode("");
      }}
      noValidate
    >
      <div className="auth__intro">
        <h1 className="display">{t("onboarding.join.twoFactor.title")}</h1>
        <p className="lede">{t("onboarding.join.twoFactor.body")}</p>
      </div>
      <TextField
        size="lg"
        label={t("onboarding.join.twoFactor.code")}
        inputMode="numeric"
        autoComplete="one-time-code"
        mono
        autoFocus
        value={code}
        onChange={(event) => {
          setCode(event.target.value);
        }}
      />
      <ErrorAlert error={error} />
      <HoloButton type="submit" size="lg" block disabled={busy}>
        {t("onboarding.join.twoFactor.submit")}
      </HoloButton>
      <Button variant="ghost" size="lg" block className="button--muted" onClick={onBack}>
        {t("action.back")}
      </Button>
    </form>
  );
}

interface Fields {
  serverUrl: string;
  email: string;
  password: string;
  secretKey: string;
  deviceName: string;
}

type SetField = (key: keyof Fields, value: string) => void;

function ServerFields({ fields, set }: { fields: Fields; set: SetField }) {
  const { t } = useI18n();
  return (
    <>
      <TextField
        size="lg"
        label={t("onboarding.join.serverUrl")}
        type="url"
        autoComplete="url"
        value={fields.serverUrl}
        onChange={(event) => {
          set("serverUrl", event.target.value);
        }}
      />
      <TextField
        size="lg"
        label={t("onboarding.join.email")}
        type="email"
        autoComplete="email"
        value={fields.email}
        onChange={(event) => {
          set("email", event.target.value);
        }}
      />
    </>
  );
}

function FolderFields({
  choice,
  accountId,
  onChoose,
  onAccount,
}: {
  choice: FolderChoice | null;
  accountId: string;
  onChoose: () => void;
  onAccount: (id: string) => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <div className="actions">
        <Button onClick={onChoose}>{t("onboarding.join.chooseFolder")}</Button>
      </div>
      {choice !== null && (
        <p className="body">
          <span className="muted">{t("onboarding.join.folder")}: </span>
          <UserText>{choice.displayPath}</UserText>
        </p>
      )}
      {choice?.accounts.length === 0 && <p className="alert">{t("onboarding.join.noAccounts")}</p>}
      {choice !== null && choice.accounts.length > 1 && (
        <Select
          label={t("onboarding.join.account")}
          value={accountId}
          options={choice.accounts.map((id) => ({
            value: id,
            label: t("onboarding.join.accountOption", { id }),
          }))}
          onChange={(event) => {
            onAccount(event.target.value);
          }}
        />
      )}
    </>
  );
}

function SecretFields({ fields, set }: { fields: Fields; set: SetField }) {
  const { t } = useI18n();
  return (
    <>
      <PasswordField
        size="lg"
        toggleLabel="password"
        label={t("onboarding.join.password")}
        autoComplete="current-password"
        value={fields.password}
        onChange={(event) => {
          set("password", event.target.value);
        }}
      />
      <PasswordField
        size="lg"
        label={t("onboarding.join.secretKey")}
        hint={t("onboarding.join.secretKeyHint")}
        autoComplete="off"
        spellCheck={false}
        value={fields.secretKey}
        onChange={(event) => {
          set("secretKey", event.target.value);
        }}
      />
      <TextField
        size="lg"
        label={t("onboarding.join.deviceName")}
        autoComplete="off"
        value={fields.deviceName}
        onChange={(event) => {
          set("deviceName", event.target.value);
        }}
      />
    </>
  );
}

/** The join calls and what they leave: the busy flag, the error and the code step. */
function useJoinFlow(onDone: (session: SessionState) => void) {
  const ipc = useIpc();
  const [needsCode, setNeedsCode] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function run(call: () => Promise<IpcResult<JoinOutcome>>) {
    setBusy(true);
    setError(null);
    const result = await call();
    if (!result.ok) {
      setError(result.error);
    } else if (result.value.state === "twoFactorRequired") {
      setNeedsCode(true);
    } else {
      const session = await ipc.call("session_state", null);
      if (session.ok) {
        onDone(session.value);
      } else {
        setError(session.error);
      }
    }
    setBusy(false);
  }
  return { needsCode, setNeedsCode, busy, error, setError, run };
}

type Flow = ReturnType<typeof useJoinFlow>;

/** The form's state and the two join requests (server or folder). */
function useJoinForm(flow: Flow) {
  const ipc = useIpc();
  const defaultName = useDefaultDeviceName();
  const [tab, setTab] = useState<Tab>("server");
  const [fields, setFields] = useState<Fields>({
    serverUrl: "",
    email: "",
    password: "",
    secretKey: "",
    deviceName: defaultName,
  });
  const [choice, setChoice] = useState<FolderChoice | null>(null);
  const [accountId, setAccountId] = useState("");
  const set: SetField = (key, value) => {
    setFields((current) => ({ ...current, [key]: value }));
  };

  async function chooseFolder() {
    flow.setError(null);
    const result = await ipc.call("sync_pick_folder", null);
    if (!result.ok) {
      flow.setError(result.error);
    } else if (result.value !== null) {
      setChoice(result.value);
      setAccountId(result.value.accounts[0] ?? "");
    }
  }

  function submit(event: SyntheticEvent) {
    event.preventDefault();
    const { serverUrl, email, password, secretKey } = fields;
    const deviceName = fields.deviceName.trim();
    if (tab === "server") {
      void flow.run(() =>
        ipc.call("join_hosted", {
          serverUrl: serverUrl.trim(),
          email: email.trim(),
          password,
          secretKey,
          deviceName,
        }),
      );
    } else if (choice !== null) {
      void flow.run(() =>
        ipc.call("join_folder", { choiceId: choice.choiceId, accountId, password, secretKey, deviceName }),
      );
    }
  }

  const noAccount = tab === "folder" && (choice === null || choice.accounts.length === 0);
  return { tab, setTab, fields, set, choice, accountId, setAccountId, chooseFolder, submit, noAccount };
}

function JoinForm({ flow, onBack }: { flow: Flow; onBack: () => void }) {
  const { t } = useI18n();
  const tabsId = useId();
  const form = useJoinForm(flow);
  return (
    <>
      <div className="auth__intro">
        <h1 className="display">{t("onboarding.join.title")}</h1>
        <p className="lede">{t("onboarding.join.body")}</p>
      </div>
      <Tabs
        id={tabsId}
        tab={form.tab}
        onTab={(next) => {
          form.setTab(next);
          flow.setError(null);
        }}
      />
      <form
        id={`${tabsId}-panel`}
        role="tabpanel"
        aria-labelledby={`${tabsId}-${form.tab}`}
        className="form-stack"
        onSubmit={form.submit}
        noValidate
      >
        {form.tab === "server" ? (
          <ServerFields fields={form.fields} set={form.set} />
        ) : (
          <FolderFields
            choice={form.choice}
            accountId={form.accountId}
            onChoose={() => {
              void form.chooseFolder();
            }}
            onAccount={form.setAccountId}
          />
        )}
        <SecretFields fields={form.fields} set={form.set} />
        <ErrorAlert error={flow.error} />
        <p className="status" aria-live="polite">
          {flow.busy ? t("onboarding.join.working") : ""}
        </p>
        <HoloButton type="submit" size="lg" block disabled={flow.busy || form.noAccount}>
          {t("onboarding.join.submit")}
        </HoloButton>
        <Button variant="ghost" size="lg" block className="button--muted" onClick={onBack}>
          {t("onboarding.join.back")}
        </Button>
      </form>
    </>
  );
}

export function JoinStep({
  onBack,
  onDone,
}: {
  onBack: () => void;
  onDone: (session: SessionState) => void;
}) {
  const ipc = useIpc();
  const flow = useJoinFlow(onDone);
  return (
    <AuthShell>
      <div className="auth__main">
        {flow.needsCode ? (
          <CodeForm
            busy={flow.busy}
            error={flow.error}
            onSubmit={(code) => {
              void flow.run(() => ipc.call("join_hosted_two_factor", { code }));
            }}
            onBack={() => {
              flow.setNeedsCode(false);
              flow.setError(null);
            }}
          />
        ) : (
          <JoinForm flow={flow} onBack={onBack} />
        )}
      </div>
    </AuthShell>
  );
}
