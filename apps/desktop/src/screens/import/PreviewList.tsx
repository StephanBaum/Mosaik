/** The import preview: warnings, a tick list of entries and the entries that are left out. */
import { Chip } from "../../components/Chip.tsx";
import { Icon } from "../../components/icons.tsx";
import { Switch } from "../../components/Switch.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { ImportEntry, ImportPreview, ImportWarning } from "../../ipc/contract/index.ts";
import { UserText } from "../../components/UserText.tsx";

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
    <li className="row entries__item">
      <label className="check entries__label">
        <input
          type="checkbox"
          checked={checked}
          onChange={(event) => {
            onToggle(event.target.checked);
          }}
        />
        <span>
          <strong>
            <UserText>{entry.title ?? t("vault.untitled")}</UserText>
          </strong>{" "}
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

/** The warnings about passkeys that were left out (the file's items are imported without them). */
const PASSKEY_WARNINGS: ReadonlySet<ImportWarning> = new Set([
  "passkeysNotImported",
  "passkeyUnreadable",
  "passkeyUnsupported",
  "passkeysSkipped",
]);

function Summary({ preview }: { preview: ImportPreview }) {
  const { t } = useI18n();
  const kinds = new Map<ImportEntry["kind"], number>();
  for (const entry of preview.entries) {
    kinds.set(entry.kind, (kinds.get(entry.kind) ?? 0) + 1);
  }
  return (
    <>
      <p className="body">
        {t("import.preview.ready", { count: preview.entries.length })}{" "}
        <span className="muted">{t("import.preview.attachments", { count: preview.attachments })}</span>
      </p>
      <div className="import-summary">
        {[...kinds].map(([kind, count]) => (
          <Chip key={kind}>
            {t(`item.kind.${kind}`)} {count}
          </Chip>
        ))}
      </div>
      {preview.warnings.length > 0 && (
        <ul className="rows rows--closed">
          {preview.warnings.map((warning) => (
            <li key={warning} className="row row--warning">
              <Icon name="alert" />
              <span className="row__warning-title">{t(`import.warning.${warning}`)}</span>
            </li>
          ))}
        </ul>
      )}
      {preview.warnings.some((warning) => PASSKEY_WARNINGS.has(warning)) && (
        <p className="field__hint">{t("import.warning.passkeysKeepOldApp")}</p>
      )}
    </>
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
      <Summary preview={preview} />
      <Switch
        label={t("import.flow.selectAll")}
        checked={all}
        onChange={(event) => {
          onChange(event.target.checked ? new Set(preview.entries.map((e) => e.index)) : new Set());
        }}
      />
      <ul className="rows rows--closed entries" aria-label={t("import.flow.entries")}>
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
          <ul className="rows rows--closed">
            {preview.skipped.map((entry) => (
              <li key={entry.index} className="row">
                {t(`import.skip.${entry.reason}`)}
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}
