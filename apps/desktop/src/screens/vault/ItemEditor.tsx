/**
 * Create or edit an item (D10): title, the kind's fields (`kinds/layouts.ts`), websites,
 * one-time code, SSH key, tags, notes and custom fields. The kind can be chosen only while
 * creating: core has no setter for it and two devices could pick different kinds (ADR-0033).
 */
import {
  useState,
  type Dispatch,
  type KeyboardEvent,
  type SetStateAction,
  type SyntheticEvent,
} from "react";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Select } from "../../components/Select.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import {
  itemKinds,
  type IpcError,
  type ItemDetail,
  type ItemKind,
  type ItemRequest,
  type SecretFieldKey,
  type VaultSummary,
} from "../../ipc/contract/index.ts";
import { Sections } from "./editor/Sections.tsx";
import { changeKind, hasErrors, initialForm, validateForm, type Form } from "./editorForm.ts";
import { OTP_UNCHANGED } from "./otp/OtpSection.tsx";
import { useSave } from "./useSave.ts";

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
  onKind,
}: {
  form: Form;
  creating: boolean;
  vaults: readonly VaultSummary[];
  onChange: (patch: Partial<Form>) => void;
  onKind: (kind: ItemKind) => void;
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
              onKind(event.target.value as ItemKind);
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

/** Asks the generator for a password and puts it in the form field `key`. */
function useFillGenerated(
  setForm: Dispatch<SetStateAction<Form>>,
  setError: (error: IpcError | null) => void,
): (key: SecretFieldKey) => void {
  const ipc = useIpc();
  return (key) => {
    void ipc.call("generate_password", GENERATED_PASSWORD).then((result) => {
      if (result.ok) {
        const value = result.value.value;
        setForm((current) => ({
          ...current,
          secrets: { ...current.secrets, [key]: value },
          clearSecrets: current.clearSecrets.filter((each) => each !== key),
        }));
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

/** The form's state, what blocks saving and what a submit does. */
function useEditorState(
  editing: ItemDetail | null,
  vaults: readonly VaultSummary[],
  vaultId: string | null,
  onSaved: (item: ItemRequest) => void,
) {
  const [initial] = useState(() => initialForm(editing, vaults, vaultId));
  const [form, setForm] = useState(initial);
  const [tried, setTried] = useState(false);
  const change = (patch: Partial<Form>) => {
    setForm((current) => ({ ...current, ...patch }));
  };
  const { error, setError, save } = useSave({
    editing,
    vaults,
    initial,
    onSaved,
    onCodeStored: () => {
      change({ otp: OTP_UNCHANGED });
    },
  });
  const errors = validateForm(form, initial);
  // A one-time code item without a code would be empty.
  const otpMissing = form.kind === "totp" && editing === null && form.otp.kind !== "pending";
  const submit = () => {
    setTried(true);
    if (!otpMissing && !hasErrors(errors)) {
      void save(form);
    }
  };
  return {
    initial,
    form,
    errors,
    tried,
    otpMissing,
    error,
    change,
    submit,
    dirty: JSON.stringify(form) !== JSON.stringify(initial),
    setKind: (kind: ItemKind) => {
      setForm((current) => changeKind(current, kind));
    },
    fillGenerated: useFillGenerated(setForm, setError),
  };
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
  const state = useEditorState(editing, vaults, vaultId, onSaved);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const { form, dirty } = state;
  const cancel = useCancel(form, onCancel);
  const heading = useHeading(editing, form.kind);
  return (
    <form
      className="editor"
      onSubmit={(event: SyntheticEvent) => {
        event.preventDefault();
        state.submit();
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
        <KindAndVault
          form={form}
          creating={editing === null}
          vaults={vaults}
          onChange={state.change}
          onKind={state.setKind}
        />
      </div>
      <Sections
        form={form}
        initial={state.initial}
        editing={editing}
        vaults={vaults}
        errors={state.errors}
        tried={state.tried}
        otpMissing={state.otpMissing}
        onChange={state.change}
        onGenerate={state.fillGenerated}
      />
      <ErrorAlert error={state.error} />
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
