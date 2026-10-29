/** The import preview: warnings, a tick list of entries and the entries that are left out. */
import { Switch } from "../../components/Switch.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { ImportEntry, ImportPreview } from "../../ipc/contract/index.ts";

function EntryRow({
  entry,
  checked,
  onToggle,
}: {
  entry: ImportEntry;
  checked: boolean;
  onToggle: (on: boolean) => void;
}) {
  const { t } = useI18n();
  return (
    <li className="entries__item">
      <label className="entries__label">
        <input
          type="checkbox"
          checked={checked}
          onChange={(event) => {
            onToggle(event.target.checked);
          }}
        />
        <span>
          <strong>{entry.title ?? t("vault.untitled")}</strong>{" "}
          <span className="muted">{entry.subtitle ?? t(`item.kind.${entry.kind}`)}</span>
        </span>
      </label>
      {entry.duplicateOf !== null && <p className="field__hint">{t("import.preview.duplicate")}</p>}
      {entry.duplicateInFile !== null && (
        <p className="field__hint">{t("import.preview.duplicateInFile")}</p>
      )}
      {entry.warnings.map((warning) => (
        <p key={warning} className="field__hint">
          {t(`import.entry.${warning}`)}
        </p>
      ))}
    </li>
  );
}

export function PreviewList({
  preview,
  selected,
  onChange,
}: {
  preview: ImportPreview;
  selected: ReadonlySet<number>;
  onChange: (next: ReadonlySet<number>) => void;
}) {
  const { t } = useI18n();
  const all = preview.entries.length > 0 && selected.size === preview.entries.length;

  function toggle(index: number, on: boolean) {
    const next = new Set(selected);
    if (on) {
      next.add(index);
    } else {
      next.delete(index);
    }
    onChange(next);
  }

  return (
    <div className="stack">
      <p className="body">
        {t("import.preview.ready", { count: preview.entries.length })}{" "}
        <span className="muted">{t("import.preview.attachments", { count: preview.attachments })}</span>
      </p>
      {preview.warnings.length > 0 && (
        <ul className="notes">
          {preview.warnings.map((warning) => (
            <li key={warning}>{t(`import.warning.${warning}`)}</li>
          ))}
        </ul>
      )}
      <Switch
        label={t("import.flow.selectAll")}
        checked={all}
        onChange={(event) => {
          onChange(event.target.checked ? new Set(preview.entries.map((e) => e.index)) : new Set());
        }}
      />
      <ul className="entries" aria-label={t("import.flow.entries")}>
        {preview.entries.map((entry) => (
          <EntryRow
            key={entry.index}
            entry={entry}
            checked={selected.has(entry.index)}
            onToggle={(on) => {
              toggle(entry.index, on);
            }}
          />
        ))}
      </ul>
      {preview.skipped.length > 0 && (
        <>
          <h2 className="subtitle">{t("import.flow.skipped")}</h2>
          <ul className="notes">
            {preview.skipped.map((entry) => (
              <li key={entry.index}>{t(`import.skip.${entry.reason}`)}</li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}
