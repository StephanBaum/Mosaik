/**
 * The custom fields of an item ("More fields"): rename, retype, set a value or remove.
 * A stored hidden field starts empty, which keeps its value; it never comes back to the page.
 */
import type { InputHTMLAttributes } from "react";
import { Button } from "../../../components/Button.tsx";
import { Icon } from "../../../components/icons.tsx";
import { IconButton } from "../../../components/IconButton.tsx";
import { PasswordField } from "../../../components/PasswordField.tsx";
import { Select } from "../../../components/Select.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import { customFieldKinds, type CustomFieldKind } from "../../../ipc/contract/index.ts";
import { newCustomRow, type CustomRow, type CustomRowError, type FormErrors } from "../editorForm.ts";

/** New fields offer no one-time code: the item's own one-time code covers it. */
const NEW_KINDS = customFieldKinds.filter((kind) => kind !== "otp");

const INPUT_TYPES: Partial<Record<CustomFieldKind, InputHTMLAttributes<HTMLInputElement>["type"]>> = {
  url: "url",
  email: "email",
  phone: "tel",
  date: "date",
};

// eslint-disable-next-line no-control-regex -- C0 and C1 control characters are refused in labels.
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/g;

function ValueInput({
  row,
  error,
  onChange,
}: {
  row: CustomRow;
  error: string | undefined;
  onChange: (value: string) => void;
}) {
  const { t } = useI18n();
  if (row.kind === "otp") {
    return <p className="field__hint">{t("editor.custom.otpReadOnly")}</p>;
  }
  const common = {
    label: t("editor.custom.value"),
    value: row.value,
    autoComplete: "off",
    error,
    onChange: (event: { target: { value: string } }) => {
      onChange(event.target.value);
    },
  };
  if (row.kind === "hidden") {
    return (
      <PasswordField
        {...common}
        hint={row.stored && row.value === "" ? t("editor.keepSecret") : undefined}
      />
    );
  }
  return <TextField {...common} type={INPUT_TYPES[row.kind] ?? "text"} />;
}

function Row({
  row,
  index,
  error,
  onChange,
  onRemove,
}: {
  row: CustomRow;
  /** 1-based position, names a row whose label is still empty. */
  index: number;
  error: CustomRowError | undefined;
  onChange: (change: Partial<CustomRow>) => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();
  const kinds = row.storedKind === "otp" ? customFieldKinds : NEW_KINDS;
  const label =
    error === "labelRequired"
      ? t("editor.custom.labelRequired")
      : error === "labelTooLong"
        ? t("editor.custom.labelTooLong")
        : undefined;
  return (
    <div
      className="row row--editor"
      role="group"
      aria-label={row.label.trim() || t("editor.custom.unnamed", { index })}
    >
      <div className="row__main">
        <TextField
          label={t("editor.custom.label")}
          value={row.label}
          autoComplete="off"
          error={label}
          onChange={(event) => {
            onChange({ label: event.target.value.replace(CONTROL, "") });
          }}
        />
      </div>
      <Select
        label={t("editor.custom.kind")}
        value={row.kind}
        disabled={row.storedKind === "otp"}
        options={kinds.map((kind) => ({ value: kind, label: t(`item.customKind.${kind}`) }))}
        onChange={(event) => {
          onChange({ kind: event.target.value as CustomFieldKind });
        }}
      />
      <div className="row__main">
        <ValueInput
          row={row}
          error={error === "kindNeedsValue" ? t("editor.custom.kindNeedsValue") : undefined}
          onChange={(value) => {
            onChange({ value });
          }}
        />
      </div>
      <IconButton
        icon="trash"
        label={t("editor.custom.remove", { label: row.label.trim() || t("editor.custom.label") })}
        onClick={onRemove}
      />
    </div>
  );
}

export function CustomFieldsEditor({
  rows,
  errors,
  canAdd,
  onChange,
}: {
  rows: readonly CustomRow[];
  errors: FormErrors["custom"];
  canAdd: boolean;
  onChange: (rows: CustomRow[]) => void;
}) {
  const { t } = useI18n();
  return (
    <section className="editor" aria-label={t("item.customFields")}>
      {rows.length > 0 && (
        <div className="rows">
          {rows.map((row, at) => (
            <Row
              key={row.rowKey}
              row={row}
              index={at + 1}
              error={errors[row.rowKey]}
              onChange={(change) => {
                onChange(rows.map((each) => (each.rowKey === row.rowKey ? { ...each, ...change } : each)));
              }}
              onRemove={() => {
                onChange(rows.filter((each) => each.rowKey !== row.rowKey));
              }}
            />
          ))}
        </div>
      )}
      <Button
        disabled={!canAdd}
        onClick={() => {
          onChange([...rows, newCustomRow()]);
        }}
      >
        <Icon name="plus" />
        {t("editor.custom.add")}
      </Button>
      {!canAdd && <p className="field__hint">{t("editor.custom.limit")}</p>}
    </section>
  );
}
