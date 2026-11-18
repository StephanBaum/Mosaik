/**
 * Import from another manager (prototype 4229). The source names, file names and counts are
 * fixed literals; the demo cycles three sources and rests on the last. Rendered in its final
 * frame (the third source, imported); `src/demos/importer.ts` replays the story.
 */
import { Icon } from "../Icon.tsx";
import type { Ctx } from "../i18n.ts";

export const IMPORT_SOURCES = [
  ["Chrome", "Chrome Passwords.csv", 184, 1],
  ["1Password", "1Password.1pux", 312, 3],
  ["LastPass", "lastpass_export.csv", 96, 2],
] as const;

const ROWS = [
  ["G", "GitHub", "alex@example.com", false],
  ["F", "Figma", "alex@example.com", false],
  ["J", "Jira", "alex@acme.example", true],
  ["W", "Wi-Fi", "Mosaic-5G", false],
] as const;

export function ImportDemo({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const last = IMPORT_SOURCES.length - 1;
  const [, file, count, dups] = IMPORT_SOURCES[last] ?? IMPORT_SOURCES[0];
  return (
    <div className="di" data-island="demos" data-demo="import" role="img" aria-label={t("web.shot.import")}>
      <div className="di__head" aria-hidden="true">
        <span>{t("web.switch.bring.title")}</span>
        <span className="di__srcs">
          {IMPORT_SOURCES.map(([name], i) => (
            <span key={name} className={i === last ? "di__src js-src is-on" : "di__src js-src"}>
              {name}
            </span>
          ))}
        </span>
      </div>
      <div
        className="di__drop js-drop"
        data-text-drop={t("web.tools.import.drop")}
        data-text-reading={t("web.tools.import.reading")}
        data-text-preview={t("web.tools.import.preview")}
        aria-hidden="true"
      >
        <Icon name="import" />
        <span className="js-drop-text">
          {t("web.tools.import.preview")} · {file}
        </span>
        <span className="di__file js-file">
          <Icon name="file" />
          <span className="js-file-name">{file}</span>
        </span>
        <i className="di__bar js-bar" />
      </div>
      <div className="di__list" aria-hidden="true">
        {ROWS.map(([initial, name, sub, dup]) => (
          <div key={name} className={dup ? "di__row js-row is-dup" : "di__row js-row"}>
            <span className="fill__avatar">{initial}</span>
            <span>
              {name}
              <small>{sub}</small>
            </span>
            <span className="ds__chip">
              <Icon name="warning" />
              {t("web.tools.import.dup")}
            </span>
          </div>
        ))}
      </div>
      <div className="di__foot" aria-hidden="true">
        <span>
          <span className="js-count">{count}</span> {t("web.tools.import.items")} · <span className="js-dups">{dups}</span> {t("web.tools.import.dups")}
        </span>
        <span className="ds__btn js-import-btn">{t("web.tools.import.button")}</span>
      </div>
      <div className="di__done js-done" aria-hidden="true">
        <Icon name="check" />
        <span>{t("web.tools.import.done")}</span>
      </div>
    </div>
  );
}
