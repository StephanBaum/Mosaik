/** The editor's sections below the header, in the order of D10. */
import { TextArea } from "../../../components/TextArea.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { ItemDetail, SecretFieldKey, VaultSummary } from "../../../ipc/contract/index.ts";
import { canAddCustomField, type Form, type FormErrors } from "../editorForm.ts";
import { hasOtpSection, hasSshKey, hasUrls } from "../kinds/layouts.ts";
import { OtpSection } from "../otp/OtpSection.tsx";
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
      <KindFields
        form={form}
        tried={props.tried}
        errors={errors}
        storedSecrets={editing?.secrets ?? []}
        onChange={onChange}
        onGenerate={props.onGenerate}
      />
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

