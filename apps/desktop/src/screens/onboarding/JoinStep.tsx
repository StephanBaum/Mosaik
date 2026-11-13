/**
 * "I already use Mosaic": add this device to an existing account (M6). The account's data comes
 * from a Mosaic server or an own cloud folder; the master password and the Secret Key from the
 * Emergency Kit open it. The joined account is already kit-verified, so onboarding ends here.
 */
import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type SyntheticEvent,
} from "react";
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
import { SecondFactorStep } from "../sync/SecondFactorStep.tsx";
import { CodeForm, Tabs, type Tab } from "./JoinParts.tsx";

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
      {choice?.accounts.length === 0 && (
        <ErrorAlert error={{ code: "syncFolderNoAccount" }} message={t("onboarding.join.noAccounts")} />
      )}
      {choice !== null && choice.accounts.length > 1 && (
        <Select
          label={t("onboarding.join.account")}
          value={accountId}
          options={choice.accounts.map((id) => ({
            value: id,
            // The first eight characters tell the accounts of one folder apart.
            label: t("onboarding.join.accountOption", { id: id.slice(0, 8) }),
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
      await finish();
    }
    setBusy(false);
  }

  /** The join is done in Rust (a code, or a security key in the browser): read the session. */
  async function finish() {
    const session = await ipc.call("session_state", null);
    if (session.ok) {
      onDone(session.value);
    } else {
      setError(session.error);
    }
  }
  return { needsCode, setNeedsCode, busy, error, setError, run, finish };
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
  const clearPassword = useCallback(() => {
    setFields((current) => ({ ...current, password: "" }));
  }, []);

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
  return {
    tab,
    setTab,
    fields,
    set,
    clearPassword,
    choice,
    accountId,
    setAccountId,
    chooseFolder,
    submit,
    noAccount,
  };
}

type Form = ReturnType<typeof useJoinForm>;

function JoinForm({ flow, form, onBack }: { flow: Flow; form: Form; onBack: () => void }) {
  const { t } = useI18n();
  const tabsId = useId();
  const heading = useRef<HTMLHeadingElement>(null);
  // The screen changed (from the welcome step, or back from the code step): start at its title.
  useEffect(() => {
    heading.current?.focus();
  }, []);
  return (
    <>
      <div className="auth__intro">
        <h1 className="display" ref={heading} tabIndex={-1}>
          {t("onboarding.join.title")}
        </h1>
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
  const { t } = useI18n();
  const ipc = useIpc();
  const flow = useJoinFlow(onDone);
  // The form lives here, so the fields survive the code step; only the password is cleared.
  const form = useJoinForm(flow);
  const { clearPassword } = form;
  // The password does not wait in memory while the code is asked for; the other fields stay.
  useEffect(() => {
    if (flow.needsCode) {
      clearPassword();
    }
  }, [flow.needsCode, clearPassword]);
  const back = () => {
    flow.setNeedsCode(false);
    flow.setError(null);
  };
  return (
    <AuthShell>
      <div className="auth__main">
        {flow.needsCode ? (
          <>
            {/* The page keeps its h1; the step below it is an h2. */}
            <div className="auth__intro">
              <h1 className="display">{t("onboarding.join.title")}</h1>
            </div>
            <SecondFactorStep
              join
              large
              onSignedIn={() => {
                void flow.finish();
              }}
              onCancel={back}
            >
              <CodeForm
                busy={flow.busy}
                error={flow.error}
                onSubmit={(code) => {
                  void flow.run(() => ipc.call("join_hosted_two_factor", { code }));
                }}
                onBack={back}
              />
            </SecondFactorStep>
          </>
        ) : (
          <JoinForm flow={flow} form={form} onBack={onBack} />
        )}
      </div>
    </AuthShell>
  );
}
