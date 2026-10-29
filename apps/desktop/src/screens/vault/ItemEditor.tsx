/** Create or edit an item (D10): title, user name, password (the generator can fill it), website, notes, tags. */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Select } from "../../components/Select.tsx";
import { TextArea } from "../../components/TextArea.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import {
  itemKinds,
  type IpcError,
  type ItemDetail,
  type ItemDraft,
  type ItemKind,
  type ItemRequest,
  type VaultSummary,
} from "../../ipc/contract/index.ts";

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

interface Form {
  kind: ItemKind;
  vault: string;
  title: string;
  username: string;
  password: string;
  url: string;
  notes: string;
  tags: string;
}

function textOf(detail: ItemDetail | null, field: string): string {
  return detail?.text.find((entry) => entry.field === field)?.value ?? "";
}

function initialForm(editing: ItemDetail | null, vaults: readonly VaultSummary[], vaultId: string | null): Form {
  return {
    kind: editing?.summary.kind ?? "login",
    vault: vaultId ?? vaults[0]?.vaultId ?? "",
    title: editing?.summary.title ?? "",
    username: textOf(editing, "username"),
    password: "",
    url: "",
    notes: textOf(editing, "notes"),
    tags: editing?.summary.tags.join(", ") ?? "",
  };
}

function buildDraft(form: Form): ItemDraft {
  const empty = (value: string): string | null => (value === "" ? null : value);
  return {
    text: [
      { field: "title", value: form.title },
      { field: "username", value: empty(form.username) },
      { field: "notes", value: empty(form.notes) },
    ],
    secrets: form.password === "" ? [] : [{ field: "password", value: form.password }],
    urls: form.url === "" ? [] : [{ op: "add", url: form.url, matchMode: "exactOrigin", allowHttp: false }],
    customFields: [],
    tags: form.tags
      .split(",")
      .map((tag) => tag.trim())
      .filter((tag) => tag !== ""),
    otp: null,
  };
}

function useSave(editing: ItemDetail | null, onSaved: (item: ItemRequest) => void) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [error, setError] = useState<IpcError | null>(null);

  async function save(form: Form) {
    setError(null);
    const draft = buildDraft(form);
    const result =
      editing === null
        ? await ipc.call("add_item", { vaultId: form.vault, kind: form.kind, draft })
        : await ipc.call("update_item", {
            vaultId: editing.summary.vaultId,
            itemId: editing.summary.itemId,
            draft,
          });
    if (result.ok) {
      toast(t("toast.saved"));
      onSaved({ vaultId: result.value.summary.vaultId, itemId: result.value.summary.itemId });
    } else {
      setError(result.error);
    }
  }
  return { error, setError, save };
}

function KindAndVault({
  form,
  vaults,
  onChange,
}: {
  form: Form;
  vaults: readonly VaultSummary[];
  onChange: (patch: Partial<Form>) => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <Select
        label={t("editor.kind")}
        value={form.kind}
        options={CREATABLE.map((each) => ({ value: each, label: t(`item.kind.${each}`) }))}
        onChange={(event) => {
          onChange({ kind: event.target.value as ItemKind });
        }}
      />
      <Select
        label={t("editor.vault")}
        value={form.vault}
        options={vaults.map((each) => ({ value: each.vaultId, label: each.name ?? t("vault.unnamed") }))}
        onChange={(event) => {
          onChange({ vault: event.target.value });
        }}
      />
    </>
  );
}

function FormFields({
  form,
  creating,
  vaults,
  onChange,
  onFill,
}: {
  form: Form;
  creating: boolean;
  vaults: readonly VaultSummary[];
  onChange: (patch: Partial<Form>) => void;
  onFill: () => void;
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
      {creating && <KindAndVault form={form} vaults={vaults} onChange={onChange} />}
      {text("title", t("item.field.title"))}
      {text("username", t("item.field.username"))}
      <TextField
        label={t("item.field.password")}
        hint={creating ? undefined : t("editor.keepSecret")}
        mono
        type="password"
        autoComplete="new-password"
        value={form.password}
        onChange={(event) => {
          onChange({ password: event.target.value });
        }}
      />
      <Button onClick={onFill}>{t("editor.fillGenerated")}</Button>
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
      <TextArea
        label={t("item.field.notes")}
        value={form.notes}
        autoComplete="off"
        onChange={(event) => {
          onChange({ notes: event.target.value });
        }}
      />
      {text("tags", t("editor.tags"))}
    </>
  );
}

function DiscardDialog({ onKeep, onDiscard }: { onKeep: () => void; onDiscard: () => void }) {
  const { t } = useI18n();
  return (
    <Dialog title={t("editor.discard.title")} onClose={onKeep}>
      <p>{t("editor.discard.body")}</p>
      <div className="actions">
        <Button autoFocus onClick={onKeep}>
          {t("editor.discard.keep")}
        </Button>
        <Button onClick={onDiscard}>{t("editor.discard.confirm")}</Button>
      </div>
    </Dialog>
  );
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
  const ipc = useIpc();
  const [initial] = useState(() => initialForm(editing, vaults, vaultId));
  const [form, setForm] = useState(initial);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const dirty = JSON.stringify(form) !== JSON.stringify(initial);
  const { error, setError, save } = useSave(editing, onSaved);

  async function fillGenerated() {
    const result = await ipc.call("generate_password", GENERATED_PASSWORD);
    if (result.ok) {
      setForm((current) => ({ ...current, password: result.value.value }));
    } else {
      setError(result.error);
    }
  }

  const heading =
    editing === null
      ? t("editor.newTitle", { kind: t(`item.kind.${form.kind}`) })
      : t("editor.editTitle", { title: editing.summary.title ?? t("vault.untitled") });
  return (
    <form
      className="editor stack"
      onSubmit={(event: SyntheticEvent) => {
        event.preventDefault();
        void save(form);
      }}
      noValidate
      onKeyDown={(event) => {
        // A dialog inside the form (the discard prompt) handles its own Esc.
        if (event.key === "Escape" && !(event.target instanceof Element && event.target.closest("dialog"))) {
          event.preventDefault();
          if (dirty) {
            setConfirmDiscard(true);
          } else {
            onCancel();
          }
        }
      }}
    >
      <h2 className="detail__title">{heading}</h2>
      <FormFields
        form={form}
        creating={editing === null}
        vaults={vaults}
        onChange={(patch) => {
          setForm((current) => ({ ...current, ...patch }));
        }}
        onFill={() => void fillGenerated()}
      />
      <ErrorAlert error={error} />
      <div className="actions">
        <Button onClick={onCancel}>{t("action.cancel")}</Button>
        <HoloButton type="submit">{t("action.save")}</HoloButton>
      </div>
      {confirmDiscard && (
        <DiscardDialog
          onKeep={() => {
            setConfirmDiscard(false);
          }}
          onDiscard={onCancel}
        />
      )}
    </form>
  );
}
