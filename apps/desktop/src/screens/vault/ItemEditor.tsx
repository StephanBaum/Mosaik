/** Create or edit an item (D10): title, user name, password (the generator can fill it), website, notes, tags. */
import {
  useState,
  type Dispatch,
  type KeyboardEvent,
  type ReactNode,
  type SetStateAction,
  type SyntheticEvent,
} from "react";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { Select } from "../../components/Select.tsx";
import { TextArea } from "../../components/TextArea.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import {
  itemKinds,
  type IpcError,
  type ItemDetail,
  type ItemKind,
  type ItemRequest,
  type VaultSummary,
} from "../../ipc/contract/index.ts";
import { initialForm, useSave, type Form } from "./editorForm.ts";
import { OTP_UNCHANGED, OtpSection } from "./otp/OtpSection.tsx";

const CREATABLE: readonly ItemKind[] = itemKinds.filter(
  (kind) => kind !== "unsupported" && kind !== "passkey",
);

/** What "fill with a generated password" asks the generator for. */
const GENERATED_PASSWORD = {
  kind: "password",
  length: 20,
  lowercase: true,
  uppercase: true,
  digits: true,
  symbols: true,
  excludeAmbiguous: true,
} as const;

function KindAndVault({
  form,
  creating,
  vaults,
  onChange,
}: {
  form: Form;
  creating: boolean;
  vaults: readonly VaultSummary[];
  onChange: (patch: Partial<Form>) => void;
}) {
  const { t } = useI18n();
  return (
    <div className="editor__pickers">
      {creating && (
        <div className="editor__picker">
          <Select
            label={t("editor.kind")}
            value={form.kind}
            options={CREATABLE.map((each) => ({ value: each, label: t(`item.kind.${each}`) }))}
            onChange={(event) => {
              onChange({ kind: event.target.value as ItemKind });
            }}
          />
        </div>
      )}
      {creating && (
        <div className="editor__picker">
          <Select
            label={t("editor.vault")}
            value={form.vault}
            options={vaults.map((each) => ({
              value: each.vaultId,
              label: each.name ?? t("vault.unnamed"),
            }))}
            onChange={(event) => {
              onChange({ vault: event.target.value });
            }}
          />
        </div>
      )}
    </div>
  );
}

function FormFields({
  form,
  creating,
  onChange,
  onFill,
  children,
}: {
  form: Form;
  creating: boolean;
  onChange: (patch: Partial<Form>) => void;
  onFill: () => void;
  /** The one-time code section, after the website. */
  children?: ReactNode;
}) {
  const { t } = useI18n();
  const text = (key: "title" | "username" | "tags", label: string, hint?: string) => (
    <TextField
      label={label}
      hint={hint}
      value={form[key]}
      required={key === "title"}
      autoComplete="off"
      onChange={(event) => {
        onChange({ [key]: event.target.value });
      }}
    />
  );
  return (
    <>
      {text("title", t("item.field.title"))}
      {text("username", t("item.field.username"))}
      <div className="editor__password">
        <PasswordField
          label={t("item.field.password")}
          hint={creating ? undefined : t("editor.keepSecret")}
          autoComplete="new-password"
          value={form.password}
          onChange={(event) => {
            onChange({ password: event.target.value });
          }}
        />
        <Button onClick={onFill}>
          <Icon name="refresh" />
          {t("action.generate")}
        </Button>
      </div>
      {creating && (
        <TextField
          label={t("editor.url")}
          type="url"
          value={form.url}
          onChange={(event) => {
            onChange({ url: event.target.value });
          }}
        />
      )}
      {children}
      <TextArea
        label={t("item.field.notes")}
        value={form.notes}
        rows={2}
        autoComplete="off"
        onChange={(event) => {
          onChange({ notes: event.target.value });
        }}
      />
      {text("tags", t("editor.tags"))}
    </>
  );
}

/** Asks the generator for a password and puts it in the form. */
function useFillGenerated(
  setForm: Dispatch<SetStateAction<Form>>,
  setError: (error: IpcError | null) => void,
): () => void {
  const ipc = useIpc();
  return () => {
    void ipc.call("generate_password", GENERATED_PASSWORD).then((result) => {
      if (result.ok) {
        setForm((current) => ({ ...current, password: result.value.value }));
      } else {
        setError(result.error);
      }
    });
  };
}

function DiscardDialog({ onKeep, onDiscard }: { onKeep: () => void; onDiscard: () => void }) {
  const { t } = useI18n();
  return (
    <Dialog title={t("editor.discard.title")} onClose={onKeep}>
      <p>{t("editor.discard.body")}</p>
      <div className="dialog__actions">
        <Button autoFocus onClick={onKeep}>
          {t("editor.discard.keep")}
        </Button>
        <Button onClick={onDiscard}>{t("editor.discard.confirm")}</Button>
      </div>
    </Dialog>
  );
}

/** The one-time code section, fed from the form. */
function EditorOtp({
  form,
  editing,
  vaults,
  missing,
  onChange,
}: {
  form: Form;
  editing: ItemDetail | null;
  vaults: readonly VaultSummary[];
  missing: boolean;
  onChange: (patch: Partial<Form>) => void;
}) {
  return (
    <OtpSection
      kind={form.kind}
      existing={editing?.otp ?? null}
      value={form.otp}
      onChange={(otp) => {
        onChange({ otp });
      }}
      vaults={vaults}
      prefill={{ issuer: form.title, account: form.username }}
      missing={missing}
    />
  );
}

/** The pending setup is forgotten when the editor is left without saving. */
function useCancel(form: Form, onCancel: () => void): () => void {
  const ipc = useIpc();
  return () => {
    if (form.otp.kind === "pending") {
      void ipc.call("otp_pending_discard", { pendingId: form.otp.result.pending.pendingId });
    }
    onCancel();
  };
}

function useHeading(editing: ItemDetail | null, kind: ItemKind): string {
  const { t } = useI18n();
  return editing === null
    ? t("editor.newTitle", { kind })
    : t("editor.editTitle", { title: editing.summary.title ?? t("vault.untitled") });
}

/** Esc leaves the editor; a dialog inside the form handles its own Esc. */
function leaveOnEscape(event: KeyboardEvent<HTMLFormElement>, leave: () => void) {
  if (event.key === "Escape" && !(event.target instanceof Element && event.target.closest("dialog"))) {
    event.preventDefault();
    leave();
  }
}

export function ItemEditor({
  vaults,
  vaultId,
  editing,
  onSaved,
  onCancel,
}: {
  vaults: readonly VaultSummary[];
  /** The vault a new item goes to by default. */
  vaultId: string | null;
  editing: ItemDetail | null;
  onSaved: (item: ItemRequest) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [initial] = useState(() => initialForm(editing, vaults, vaultId));
  const [form, setForm] = useState(initial);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const [otpTried, setOtpTried] = useState(false);
  const dirty = JSON.stringify(form) !== JSON.stringify(initial);
  const change = (patch: Partial<Form>) => {
    setForm((current) => ({ ...current, ...patch }));
  };
  const { error, setError, save } = useSave(editing, vaults, onSaved, () => {
    change({ otp: OTP_UNCHANGED });
  });
  const cancel = useCancel(form, onCancel);
  // A one-time code item without a code would be empty.
  const otpMissing = form.kind === "totp" && editing === null && form.otp.kind !== "pending";
  const fillGenerated = useFillGenerated(setForm, setError);
  const heading = useHeading(editing, form.kind);
  return (
    <form
      className="editor"
      onSubmit={(event: SyntheticEvent) => {
        event.preventDefault();
        if (otpMissing) {
          setOtpTried(true);
          return;
        }
        void save(form);
      }}
      noValidate
      onKeyDown={(event) => {
        leaveOnEscape(event, () => {
          if (dirty) {
            setConfirmDiscard(true);
          } else {
            cancel();
          }
        });
      }}
    >
      <div className="editor__header">
        <h2 className="display">{heading}</h2>
        <KindAndVault form={form} creating={editing === null} vaults={vaults} onChange={change} />
      </div>
      <FormFields form={form} creating={editing === null} onChange={change} onFill={fillGenerated}>
        <EditorOtp
          form={form}
          editing={editing}
          vaults={vaults}
          missing={otpTried && otpMissing}
          onChange={change}
        />
      </FormFields>
      <ErrorAlert error={error} />
      <div className="form-footer">
        <Button onClick={cancel}>{t("action.cancel")}</Button>
        <HoloButton type="submit">{t("action.save")}</HoloButton>
      </div>
      {confirmDiscard && (
        <DiscardDialog
          onKeep={() => {
            setConfirmDiscard(false);
          }}
          onDiscard={cancel}
        />
      )}
    </form>
  );
}
