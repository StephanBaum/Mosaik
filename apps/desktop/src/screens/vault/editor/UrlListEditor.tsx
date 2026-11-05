/** The websites of a login: the stored ones and new rows, each with how it is matched. */
import { Button } from "../../../components/Button.tsx";
import { Icon } from "../../../components/icons.tsx";
import { IconButton } from "../../../components/IconButton.tsx";
import { Select } from "../../../components/Select.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import { urlMatchModes, type UrlMatchMode } from "../../../ipc/contract/index.ts";
import { newUrlRow, type UrlRow } from "../editorForm.ts";

export function UrlListEditor({
  rows,
  onChange,
}: {
  rows: readonly UrlRow[];
  onChange: (rows: UrlRow[]) => void;
}) {
  const { t } = useI18n();
  const patch = (rowKey: string, change: Partial<UrlRow>) => {
    onChange(rows.map((row) => (row.rowKey === rowKey ? { ...row, ...change } : row)));
  };
  return (
    <>
      <h3 className="editor__group-heading">{t("editor.urls.heading")}</h3>
      {rows.map((row, at) => (
        <div key={row.rowKey} className="editor__password">
          <TextField
            label={t("editor.urls.label", { index: at + 1 })}
            type="url"
            autoComplete="off"
            value={row.url}
            hint={at === 0 ? t("editor.urls.hint") : undefined}
            onChange={(event) => {
              patch(row.rowKey, { url: event.target.value });
            }}
          />
          <Select
            label={t("editor.urls.match", { index: at + 1 })}
            value={row.matchMode}
            options={urlMatchModes.map((mode) => ({
              value: mode,
              label: t(mode === "exactOrigin" ? "editor.urls.matchExactRecommended" : `item.urlMatch.${mode}`),
            }))}
            onChange={(event) => {
              patch(row.rowKey, { matchMode: event.target.value as UrlMatchMode });
            }}
          />
          <IconButton
            icon="trash"
            label={t("editor.urls.remove", { index: at + 1 })}
            onClick={() => {
              onChange(rows.filter((each) => each.rowKey !== row.rowKey));
            }}
          />
        </div>
      ))}
      <div>
        <Button
          onClick={() => {
            onChange([...rows, newUrlRow()]);
          }}
        >
          <Icon name="plus" />
          {t("editor.urls.add")}
        </Button>
      </div>
    </>
  );
}
