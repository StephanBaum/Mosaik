/**
 * The fields of the selected kind, from its layout (`kinds/layouts.ts`). Text fields, masked
 * secrets (empty means "keep" when editing; a button marks a stored one for removal) and the
 * Wi-Fi security select.
 */
import { Fragment } from "react";
import { Button } from "../../../components/Button.tsx";
import { Icon } from "../../../components/icons.tsx";
import { IconButton } from "../../../components/IconButton.tsx";
import { PasswordField } from "../../../components/PasswordField.tsx";
import { Select } from "../../../components/Select.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { SecretFieldKey, TextFieldKey } from "../../../ipc/contract/index.ts";
import type { Form, FormErrors } from "../editorForm.ts";
import { brandFromNumber, cardNumberHint } from "../kinds/cardFormat.ts";
import { layoutOf, WIFI_SECURITY, type FieldSpec } from "../kinds/layouts.ts";

interface Shared {
  form: Form;
  tried: boolean;
  errors: FormErrors;
  onChange: (patch: Partial<Form>) => void;
}

function labelOf(
  t: ReturnType<typeof useI18n>["t"],
  spec: FieldSpec,
): string {
  return t(spec.labelKey ?? `item.field.${spec.key}`);
}

function WifiSecurity({ form, label, onChange }: Pick<Shared, "form" | "onChange"> & { label: string }) {
  const { t } = useI18n();
  const value = form.text.wifiSecurity ?? "";
  const known: readonly string[] = WIFI_SECURITY;
  const options = [
    ...(value === "" ? [{ value: "", label }] : []),
    ...(value !== "" && !known.includes(value) ? [{ value, label: value }] : []),
    ...WIFI_SECURITY.map((each) => ({
      value: each,
      label: each === "None" ? t("editor.wifi.securityNone") : each,
    })),
  ];
  return (
    <Select
      label={label}
      value={value}
      options={options}
      onChange={(event) => {
        onChange({ text: { ...form.text, wifiSecurity: event.target.value } });
      }}
    />
  );
}

function textInputType(spec: FieldSpec): "text" | "email" | "tel" | "date" {
  return spec.input === "email" || spec.input === "tel" || spec.input === "date" ? spec.input : "text";
}

function TextInput({ spec, form, tried, errors, onChange }: Shared & { spec: FieldSpec }) {
  const { t } = useI18n();
  const key = spec.key as TextFieldKey;
  const label = labelOf(t, spec);
  if (spec.input === "select") {
    return <WifiSecurity form={form} label={label} onChange={onChange} />;
  }
  const value = form.text[key] ?? "";
  let hint: string | undefined;
  if (spec.input === "expiry") {
    hint = t("editor.card.expiryHint");
  } else if (spec.input === "email" && value !== "" && !value.includes("@")) {
    hint = t("editor.email.hint");
  }
  return (
    <TextField
      label={label}
      type={textInputType(spec)}
      hint={hint}
      error={tried && errors.cardExpiry && spec.input === "expiry" ? t("editor.card.expiryInvalid") : undefined}
      value={value}
      autoComplete="off"
      onChange={(event) => {
        onChange({ text: { ...form.text, [key]: event.target.value } });
      }}
    />
  );
}

/** The hint under a secret: why it is empty, what it will do, or what looks off. */
function secretHint(
  t: ReturnType<typeof useI18n>["t"],
  spec: FieldSpec,
  state: { typed: string; stored: boolean; clearing: boolean; label: string },
): string | undefined {
  if (state.clearing) {
    return t("editor.secretWillClear", { field: state.label });
  }
  if (spec.input === "cardNumber" && state.typed !== "") {
    const problem = cardNumberHint(state.typed);
    return problem === null ? undefined : t(problem === "length" ? "editor.card.lengthHint" : "editor.card.luhnHint");
  }
  if (state.stored && state.typed === "") {
    return t("editor.keepSecret");
  }
  return spec.key === "cardVerificationCode" ? t("editor.card.cvvHint") : undefined;
}

function SecretInput({
  spec,
  form,
  stored,
  onChange,
  onGenerate,
}: Pick<Shared, "form" | "onChange"> & {
  spec: FieldSpec;
  stored: boolean;
  onGenerate: (key: SecretFieldKey) => void;
}) {
  const { t } = useI18n();
  const key = spec.key as SecretFieldKey;
  const label = labelOf(t, spec);
  const typed = form.secrets[key] ?? "";
  const clearing = form.clearSecrets.includes(key);
  const numeric = spec.input === "digits" || spec.input === "cardNumber";
  const change = (value: string) => {
    const patch: Partial<Form> = { secrets: { ...form.secrets, [key]: value } };
    if (key === "cardNumber") {
      // The brand follows the number while it is empty or still the one the last number gave.
      const before = form.text.cardBrand ?? "";
      if (before === "" || before === brandFromNumber(typed)) {
        patch.text = { ...form.text, cardBrand: brandFromNumber(value) ?? "" };
      }
    }
    onChange(patch);
  };
  return (
    <div className="editor__password">
      <PasswordField
        label={label}
        hint={secretHint(t, spec, { typed, stored, clearing, label })}
        autoComplete={key === "password" ? "new-password" : "off"}
        inputMode={numeric ? "numeric" : undefined}
        disabled={clearing}
        value={typed}
        onChange={(event) => {
          change(event.target.value);
        }}
      />
      {spec.generator && (
        <Button
          onClick={() => {
            onGenerate(key);
          }}
        >
          <Icon name="refresh" />
          {t("action.generate")}
        </Button>
      )}
      {stored && (
        <IconButton
          icon="trash"
          label={t("editor.clearSecret", { field: label })}
          pressed={clearing}
          onClick={() => {
            onChange({
              secrets: { ...form.secrets, [key]: "" },
              clearSecrets: clearing
                ? form.clearSecrets.filter((each) => each !== key)
                : [...form.clearSecrets, key],
            });
          }}
        />
      )}
    </div>
  );
}

export function KindFields({
  storedSecrets,
  onGenerate,
  ...shared
}: Shared & {
  /** The secrets the item holds now (editing only). */
  storedSecrets: readonly SecretFieldKey[];
  onGenerate: (key: SecretFieldKey) => void;
}) {
  const { t } = useI18n();
  const layout = layoutOf(shared.form.kind);
  return layout.map((spec, at) => {
    const group = spec.group;
    return (
      <Fragment key={spec.key}>
        {group !== null && layout[at - 1]?.group !== group && (
          <h3 className="editor__group-heading">{t(`editor.section.${group}`)}</h3>
        )}
        {spec.secret ? (
          <SecretInput
            spec={spec}
            form={shared.form}
            stored={storedSecrets.includes(spec.key as SecretFieldKey)}
            onChange={shared.onChange}
            onGenerate={onGenerate}
          />
        ) : (
          <TextInput spec={spec} {...shared} />
        )}
      </Fragment>
    );
  });
}
