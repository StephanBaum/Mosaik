/**
 * The private key of an SSH key item: pasted once, masked while typing (the eye shows it),
 * and never read back. The key goes to Rust, which derives the public key and the fingerprint;
 * the webview never gets the private key again.
 */
import { useState } from "react";
import { IconButton } from "../../../components/IconButton.tsx";
import { TextArea } from "../../../components/TextArea.tsx";
import { FieldError } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { FormErrors } from "../editorForm.ts";

export function SshKeyInput({
  value,
  creating,
  error,
  onChange,
}: {
  value: string;
  creating: boolean;
  error: FormErrors["sshKey"];
  onChange: (value: string) => void;
}) {
  const { t } = useI18n();
  const [shown, setShown] = useState(false);
  const label = t("item.field.sshPrivateKey");
  return (
    <div>
      <div className="editor__password">
        <TextArea
          label={label}
          hint={t(creating ? "editor.ssh.privateKeyHint" : "editor.ssh.keepKey")}
          className={`field__input field__input--multiline ${shown ? "field__input--mono" : "field__input--secret"}`}
          rows={6}
          value={value}
          autoComplete="off"
          autoCapitalize="off"
          spellCheck={false}
          aria-invalid={error === null ? undefined : true}
          onChange={(event) => {
            onChange(event.target.value);
          }}
        />
        <IconButton
          icon={shown ? "eyeOff" : "eye"}
          label={t(shown ? "item.secret.hideAction" : "item.secret.revealAction", { field: label })}
          pressed={shown}
          onClick={() => {
            setShown((current) => !current);
          }}
        />
      </div>
      {error !== null && <FieldError id="ssh-key-error" message={t("error.sshKeyInvalid")} />}
    </div>
  );
}
