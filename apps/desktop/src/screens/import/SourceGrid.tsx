/** Step 1: where the passwords are now. Product names in text with a generic icon (no logos). */
import { Icon, type IconName } from "../../components/icons.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { importSources, type ImportSourceInfo, type SourceId } from "./sources.ts";

const GROUP_ICON: Record<ImportSourceInfo["group"], IconName> = {
  browser: "external",
  manager: "key",
  other: "file",
};

export function SourceGrid({ onPick }: { onPick: (id: SourceId) => void }) {
  const { t } = useI18n();
  const groups = [
    { group: "browser", title: t("import.grid.browsers") },
    { group: "manager", title: t("import.grid.managers") },
    { group: "other", title: t("import.grid.other") },
  ] as const;
  return (
    <div className="stack">
      <h2 className="subtitle">{t("import.grid.title")}</h2>
      {groups.map(({ group, title }) => (
        <section key={group} className="stack" aria-label={title}>
          <h3 className="caption">{title}</h3>
          <div className="source-grid">
            {importSources
              .filter((source) => source.group === group)
              .map((source) => (
                <button
                  key={source.id}
                  type="button"
                  className="source-grid__item"
                  onClick={() => {
                    onPick(source.id);
                  }}
                >
                  <Icon name={GROUP_ICON[group]} />
                  <span>{t(`import.source.${source.id}`)}</span>
                </button>
              ))}
          </div>
        </section>
      ))}
    </div>
  );
}
