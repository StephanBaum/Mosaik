/** The editor's sections below the header, in the order of D10. */
import { Callout } from "../../../components/Callout.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { TextArea } from "../../../components/TextArea.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { IpcError, ItemDetail, SecretFieldKey, VaultSummary } from "../../../ipc/contract/index.ts";
import { canAddCustomField, type Form, type FormErrors } from "../editorForm.ts";
import { agentKey, type AgentRow } from "../agentForm.ts";
import { hasOtpSection, hasSshKey, hasUrls, layoutOf } from "../kinds/layouts.ts";
import { OtpSection } from "../otp/OtpSection.tsx";
import { AgentAccessEditor } from "./AgentAccessEditor.tsx";
import { CustomFieldsEditor } from "./CustomFieldsEditor.tsx";
import { KindFields } from "./KindFields.tsx";
import { SshKeyInput } from "./SshKeyInput.tsx";
import { UrlListEditor } from "./UrlListEditor.tsx";

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
      prefill={{ issuer: form.title, account: form.text.username ?? "" }}
      missing={missing}
    />
  );
}

export interface SectionsProps {
  form: Form;
  initial: Form;
  editing: ItemDetail | null;
  vaults: readonly VaultSummary[];
  errors: FormErrors;
  tried: boolean;
  otpMissing: boolean;
  onChange: (patch: Partial<Form>) => void;
  onGenerate: (key: SecretFieldKey) => void;
  /** Agent access is on: offer the section under the values an agent could use. */
  agentEnabled: boolean;
  /** What the last save answered for agent access (shown in the section). */
  agentError: IpcError | null;
}

function NotesAndTags({ form, onChange }: Pick<SectionsProps, "form" | "onChange">) {
  const { t } = useI18n();
  const note = form.kind === "secureNote";
  const notes = (
    <TextArea
      label={t("item.field.notes")}
      value={form.notes}
      rows={note ? 12 : 2}
      autoComplete="off"
      onChange={(event) => {
        onChange({ notes: event.target.value });
      }}
    />
  );
  const tags = (
    <TextField
      label={t("editor.tags")}
      value={form.tags}
      autoComplete="off"
      onChange={(event) => {
        onChange({ tags: event.target.value });
      }}
    />
  );
  const filesAfterSave = form.kind === "document" && form.creating && (
    <p className="field__hint">{t("editor.document.filesAfterSave")}</p>
  );
  return note ? (
    <>
      {notes}
      {tags}
    </>
  ) : (
    <>
      {tags}
      {notes}
      {filesAfterSave}
    </>
  );
}

/** The kinds whose secrets are not an API key an agent could send (a passkey, an SSH key). */
const NO_AGENT_KINDS: readonly Form["kind"][] = ["passkey", "sshKey"];

/**
 * "Agent access" under the values an agent could use: the kind's secret fields and the stored
 * hidden custom fields (ADR-0159). Only while the user switched agent access on.
 */
function AgentAccessSection({ form, error, onChange }: Pick<SectionsProps, "form" | "onChange"> & { error: IpcError | null }) {
  const { t } = useI18n();
  const layout = layoutOf(form.kind);
  const textFields = layout
    .filter((spec) => !spec.secret)
    .map((spec) => ({ value: spec.key, label: t(spec.labelKey ?? `item.field.${spec.key}`) }));
  const fields = [
    ...layout
      .filter((spec) => spec.secret)
      .map((spec) => ({
        key: agentKey({ kind: "field", field: spec.key as SecretFieldKey }),
        label: t(spec.labelKey ?? `item.field.${spec.key}`),
      })),
    ...form.custom
      .filter((row) => row.kind === "hidden" && row.elementId !== null)
      .map((row) => ({ key: agentKey({ kind: "customField", elementId: row.elementId ?? "" }), label: row.label })),
  ];
  if (fields.length === 0) {
    return null;
  }
  const set = (key: string, row: AgentRow) => {
    onChange({ agent: { ...form.agent, [key]: row } });
  };
  return (
    <>
      <h3 className="editor__group-heading">{t("agent.editor.title")}</h3>
      <p className="field__hint">{t("agent.editor.shared")}</p>
      <ErrorAlert error={error} />
      {fields.map((field) => (
        <AgentAccessEditor
          key={field.key}
          label={field.label}
          row={form.agent[field.key]}
          textFields={textFields}
          onChange={(row) => {
            set(field.key, row);
          }}
        />
      ))}
    </>
  );
}

/** The agent section for the kinds that can have one. */
function AgentSlot({
  enabled,
  ...props
}: Pick<SectionsProps, "form" | "onChange"> & { enabled: boolean; error: IpcError | null }) {
  return enabled && !NO_AGENT_KINDS.includes(props.form.kind) ? <AgentAccessSection {...props} /> : null;
}

/** Why a passkey has so few fields to edit. */
function PasskeyFixed({ kind }: { kind: Form["kind"] }) {
  const { t } = useI18n();
  return kind === "passkey" ? <Callout tone="info">{t("editor.passkey.fixed")}</Callout> : null;
}

/**
 * Title, the kind's fields, websites, one-time code, SSH key, tags, notes and custom fields. A
 * passkey keeps only title, tags and notes: the rest comes from the site that made it.
 */
export function Sections(props: SectionsProps) {
  const { t } = useI18n();
  const { form, initial, editing, errors, onChange } = props;
  return (
    <>
      <TextField
        label={t("item.field.title")}
        value={form.title}
        required
        autoFocus
        autoComplete="off"
        error={props.tried && errors.titleRequired ? t("editor.titleRequired") : undefined}
        onChange={(event) => {
          onChange({ title: event.target.value });
        }}
      />
      {hasSshKey(form.kind) && (
        <SshKeyInput
          value={form.sshPrivateKey}
          creating={initial.creating}
          error={props.tried ? errors.sshKey : null}
          onChange={(sshPrivateKey) => {
            onChange({ sshPrivateKey });
          }}
        />
      )}
      <PasskeyFixed kind={form.kind} />
      <KindFields
        form={form}
        tried={props.tried}
        errors={errors}
        storedSecrets={editing?.secrets ?? []}
        onChange={onChange}
        onGenerate={props.onGenerate}
      />
      <AgentSlot enabled={props.agentEnabled} form={form} error={props.agentError} onChange={onChange} />
      {hasUrls(form.kind) && (
        <UrlListEditor
          rows={form.urls}
          onChange={(urls) => {
            onChange({ urls });
          }}
        />
      )}
      {hasOtpSection(form.kind) && (
        <EditorOtp
          form={form}
          editing={editing}
          vaults={props.vaults}
          missing={props.tried && props.otpMissing}
          onChange={onChange}
        />
      )}
      <NotesAndTags form={form} onChange={onChange} />
      {form.kind !== "passkey" && (
        <CustomFieldsEditor
          rows={form.custom}
          errors={props.tried ? errors.custom : {}}
          canAdd={canAddCustomField(form)}
          onChange={(custom) => {
            onChange({ custom });
          }}
        />
      )}
    </>
  );
}

