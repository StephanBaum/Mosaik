/**
 * The field rows of an item. The rows follow the layout of the item's kind
 * (`kinds/layouts.ts`); text and secret fields that hold a value but are not in the layout
 * (from an import, say) are listed under "Other fields", so nothing is hidden. Then the
 * one-time code, websites and custom fields.
 */
import { Fragment } from "react";
import { Chip } from "../../components/Chip.tsx";
import { CopyButton } from "../../components/CopyButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { SecretField } from "../../components/SecretField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type {
  CopyField,
  ItemDetail as Detail,
  ItemRequest,
  SecretFieldKey,
  TextFieldKey,
} from "../../ipc/contract/index.ts";
import { useNow } from "../../time/clock.ts";
import { groupCardNumber, isExpired } from "./kinds/cardFormat.ts";
import { layoutOf, type FieldSpec } from "./kinds/layouts.ts";
import { SshRows } from "./SshRows.tsx";
import { TotpCode } from "./TotpCode.tsx";

/** Title and notes are shown in the header and below the rows. */
const NOT_ROWS = new Set<string>(["title", "notes"]);

function textCopy(field: TextFieldKey): CopyField {
  return field === "username" ? { kind: "username" } : { kind: "text", field };
}

function secretCopy(field: SecretFieldKey): CopyField {
  return field === "password" ? { kind: "password" } : { kind: "secret", field };
}

/** The "Expired" chip of a card; reads the clock only for the expiry row. */
function ExpiredChip({ expiry }: { expiry: string }) {
  const { t } = useI18n();
  const now = useNow();
  return isExpired(expiry, new Date(now)) ? (
    <Chip tone="warning">
      <Icon name="alert" size="sm" />
      {t("item.card.expired")}
    </Chip>
  ) : null;
}

function TextRow({
  detail,
  item,
  field,
  copy,
}: {
  detail: Detail;
  item: ItemRequest;
  field: TextFieldKey;
  copy: boolean;
}) {
  const { t } = useI18n();
  const value = detail.text.find((entry) => entry.field === field)?.value ?? "";
  const label = t(`item.field.${field}`);
  return (
    <div className="row">
      <div className="row__main">
        <span className="row__label">{label}</span>
        <span className="row__value">{value}</span>
      </div>
      {field === "cardExpiry" && <ExpiredChip expiry={value} />}
      {copy && <CopyButton source={{ kind: "item", ...item, field: textCopy(field) }} field={label} />}
    </div>
  );
}

function SecretRow({
  item,
  field,
  copy,
  labelKey,
}: {
  item: ItemRequest;
  field: SecretFieldKey;
  copy: boolean;
  labelKey: FieldSpec["labelKey"];
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const label = t(labelKey ?? `item.field.${field}`);
  return (
    <SecretField
      label={label}
      reveal={() => ipc.call("reveal_secret", { ...item, secret: { kind: "field", field } })}
      format={field === "cardNumber" ? groupCardNumber : undefined}
      actions={
        copy ? (
          <CopyButton source={{ kind: "item", ...item, field: secretCopy(field) }} field={label} />
        ) : undefined
      }
    />
  );
}

function present(detail: Detail, spec: FieldSpec): boolean {
  return spec.secret
    ? detail.secrets.includes(spec.key as SecretFieldKey)
    : detail.text.some((entry) => entry.field === spec.key);
}

function LayoutRows({ detail, item }: { detail: Detail; item: ItemRequest }) {
  const { t } = useI18n();
  const specs = layoutOf(detail.summary.kind).filter((spec) => present(detail, spec));
  return specs.map((spec, at) => {
    const group = spec.group;
    return (
      <Fragment key={spec.key}>
        {group !== null && specs[at - 1]?.group !== group && (
          <h3 className="editor__group-heading">{t(`editor.section.${group}`)}</h3>
        )}
        {spec.secret ? (
          <SecretRow
            item={item}
            field={spec.key as SecretFieldKey}
            copy={spec.copy}
            labelKey={spec.labelKey}
          />
        ) : (
          <TextRow detail={detail} item={item} field={spec.key as TextFieldKey} copy={spec.copy} />
        )}
      </Fragment>
    );
  });
}

function OtherRows({ detail, item }: { detail: Detail; item: ItemRequest }) {
  const { t } = useI18n();
  const listed = new Set<string>(layoutOf(detail.summary.kind).map((spec) => spec.key));
  const text = detail.text.filter((entry) => !NOT_ROWS.has(entry.field) && !listed.has(entry.field));
  const secrets = detail.secrets.filter((field) => !listed.has(field));
  if (text.length + secrets.length === 0) {
    return null;
  }
  return (
    <>
      {layoutOf(detail.summary.kind).length > 0 && (
        <h3 className="editor__group-heading">{t("item.otherFields")}</h3>
      )}
      {text.map((entry) => (
        <TextRow key={entry.field} detail={detail} item={item} field={entry.field} copy />
      ))}
      {secrets.map((field) => (
        <SecretRow key={field} item={item} field={field} copy labelKey={null} />
      ))}
    </>
  );
}

function UrlRows({ detail }: { detail: Detail }) {
  const { t } = useI18n();
  return detail.urls.map((entry) => (
    <div key={entry.elementId} className="row">
      <div className="row__main">
        <span className="row__label">{t("item.urls", { count: detail.urls.length })}</span>
        <span className="row__value">{entry.url}</span>
      </div>
      <Chip>{t(`item.urlMatch.${entry.matchMode}`)}</Chip>
      {entry.allowHttp && <Chip>{t("item.urlMatch.http")}</Chip>}
    </div>
  ));
}

function CustomRows({ detail, item }: { detail: Detail; item: ItemRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  return detail.customFields.map((field) => {
    const copy = { kind: "item", ...item, field: { kind: "customField", elementId: field.elementId } } as const;
    if (field.kind === "otp") {
      return (
        <div key={field.elementId} className="row">
          <div className="row__main">
            <span className="row__label">{field.label}</span>
          </div>
          <Chip>{t("item.customKind.otp")}</Chip>
        </div>
      );
    }
    if (field.kind === "hidden") {
      return (
        <SecretField
          key={field.elementId}
          label={field.label}
          reveal={() =>
            ipc.call("reveal_secret", {
              ...item,
              secret: { kind: "customField", elementId: field.elementId },
            })
          }
          actions={<CopyButton source={copy} field={field.label} />}
        />
      );
    }
    return (
      <div key={field.elementId} className="row">
        <div className="row__main">
          <span className="row__label">{field.label}</span>
          <span className="row__value">{field.value}</span>
        </div>
        <CopyButton source={copy} field={field.label} />
      </div>
    );
  });
}

function CustomHeading() {
  const { t } = useI18n();
  return <h3 className="editor__group-heading">{t("item.customFields")}</h3>;
}

export function DetailFields({ detail, item }: { detail: Detail; item: ItemRequest }) {
  return (
    <div className="rows">
      {detail.sshKey !== null && <SshRows summary={detail.sshKey} item={item} />}
      <LayoutRows detail={detail} item={item} />
      <OtherRows detail={detail} item={item} />
      {detail.otp !== null && <TotpCode source={{ kind: "item", item }} />}
      <UrlRows detail={detail} />
      {detail.customFields.length > 0 && <CustomHeading />}
      <CustomRows detail={detail} item={item} />
    </div>
  );
}
