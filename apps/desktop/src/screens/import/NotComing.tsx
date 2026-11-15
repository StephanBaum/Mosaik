/** What an export leaves behind, shown before the commit so nothing surprises the user later. */
import type { MessageKey } from "@mosaic/i18n";
import { Icon } from "../../components/icons.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { ImportPreview, ImportWarning } from "../../ipc/contract/index.ts";
import type { ImportSourceInfo } from "./sources.ts";

export interface LostLine {
  key: MessageKey;
  values?: Record<string, string | number>;
}

const PASSKEY_WARNINGS: readonly ImportWarning[] = [
  "passkeysNotImported",
  "passkeyUnreadable",
  "passkeyUnsupported",
  "passkeysSkipped",
];

/** The source's known gaps, then what this file's preview adds. No line appears twice. */
export function notComing(preview: ImportPreview, source: ImportSourceInfo): LostLine[] {
  const lines: LostLine[] = source.lost.map((lost) => ({ key: `import.lost.${lost}` as MessageKey }));
  if (preview.skipped.length > 0) {
    lines.push({ key: "import.lost.entriesSkipped", values: { count: preview.skipped.length } });
  }
  if (preview.warnings.includes("attachmentsSkipped")) {
    lines.push({ key: "import.lost.attachmentsSkipped" });
  }
  if (preview.entries.some((entry) => entry.warnings.includes("fieldsDropped"))) {
    lines.push({ key: "import.lost.fieldsDropped" });
  }
  if (preview.entries.some((entry) => entry.warnings.includes("notesTruncated"))) {
    lines.push({ key: "import.lost.notesTruncated" });
  }
  for (const warning of PASSKEY_WARNINGS) {
    if (preview.warnings.includes(warning)) {
      lines.push({ key: `import.warning.${warning}` });
    }
  }
  return lines.filter((line, i) => lines.findIndex((other) => other.key === line.key) === i);
}

export function NotComing({ preview, source }: { preview: ImportPreview; source: ImportSourceInfo }) {
  const { t } = useI18n();
  const lines = notComing(preview, source);
  if (lines.length === 0) {
    return null;
  }
  const name = t(`import.source.${source.id}`);
  return (
    <section className="stack" aria-labelledby="import-lost-title">
      <h2 className="subtitle" id="import-lost-title">
        {t("import.lost.title")}
      </h2>
      <ul className="rows rows--closed">
        {lines.map((line) => (
          <li key={line.key} className="row row--warning">
            <Icon name="alert" />
            <span className="row__warning-title">{t(line.key, { source: name, ...line.values })}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
