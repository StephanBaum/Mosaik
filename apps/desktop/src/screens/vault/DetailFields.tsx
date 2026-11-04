/** The field rows of an item: text, secrets, one-time code, websites and custom fields. */
import { Chip } from "../../components/Chip.tsx";
import { CopyButton } from "../../components/CopyButton.tsx";
import { SecretField } from "../../components/SecretField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { ItemDetail as Detail, ItemRequest } from "../../ipc/contract/index.ts";
import { TotpCode } from "./TotpCode.tsx";

/** Title and notes are shown in the header and below the rows. */
const NOT_ROWS = new Set(["title", "notes"]);

function TextRows({ detail, item }: { detail: Detail; item: ItemRequest }) {
  const { t } = useI18n();
  return detail.text
    .filter((entry) => !NOT_ROWS.has(entry.field))
    .map((entry) => {
      const label = t(`item.field.${entry.field}`);
      return (
        <div key={entry.field} className="row">
          <div className="row__main">
            <span className="row__label">{label}</span>
            <span className="row__value">{entry.value}</span>
          </div>
          {entry.field === "username" && (
            <CopyButton source={{ kind: "item", ...item, field: { kind: "username" } }} field={label} />
          )}
        </div>
      );
    });
}

function SecretRows({ detail, item }: { detail: Detail; item: ItemRequest }) {
  const { t } = useI18n();
  const ipc = useIpc();
  return detail.secrets.map((field) => {
    const label = t(`item.field.${field}`);
    const copyField =
      field === "password" ? ({ kind: "password" } as const) : ({ kind: "secret", field } as const);
    return (
      <SecretField
        key={field}
        label={label}
        reveal={() => ipc.call("reveal_secret", { ...item, secret: { kind: "field", field } })}
        actions={<CopyButton source={{ kind: "item", ...item, field: copyField }} field={label} />}
      />
    );
  });
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
  const ipc = useIpc();
  return detail.customFields.map((field) => {
    if (field.kind === "otp") {
      return null;
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
          actions={
            <CopyButton
              source={{ kind: "item", ...item, field: { kind: "customField", elementId: field.elementId } }}
              field={field.label}
            />
          }
        />
      );
    }
    return (
      <div key={field.elementId} className="row">
        <div className="row__main">
          <span className="row__label">{field.label}</span>
          <span className="row__value">{field.value}</span>
        </div>
        <CopyButton
          source={{ kind: "item", ...item, field: { kind: "customField", elementId: field.elementId } }}
          field={field.label}
        />
      </div>
    );
  });
}

export function DetailFields({ detail, item }: { detail: Detail; item: ItemRequest }) {
  return (
    <div className="rows">
      <TextRows detail={detail} item={item} />
      <SecretRows detail={detail} item={item} />
      {detail.otp !== null && <TotpCode source={{ kind: "item", item }} />}
      <UrlRows detail={detail} />
      <CustomRows detail={detail} item={item} />
    </div>
  );
}
