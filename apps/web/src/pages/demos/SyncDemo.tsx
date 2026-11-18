/**
 * The sync tamper check (prototype 4198): the server offers an older checkpoint, {name} warns
 * and waits for the person. Rendered in its final frame; `src/demos/sync.ts` replays the story.
 */
import { Icon } from "../Icon.tsx";
import type { Ctx } from "../i18n.ts";

export function SyncDemo({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const cp = t("web.demo.sync.checkpoint");
  return (
    <div className="ds" data-island="demos" data-demo="sync" role="img" aria-label={t("web.how.warningAlt")}>
      <div className="ds__head" aria-hidden="true">
        <span>{t("web.demo.sync.panel")}</span>
        <i className="ds__led js-led is-bad" />
      </div>
      <div className="ds__rows" aria-hidden="true">
        <div className="ds__row">
          <span>
            <small>{t("web.demo.sync.device")}</small>
            <span className="ds__cp">{cp} #1842</span>
          </span>
          <span className="ds__chip">
            <Icon name="shield" />
            {t("web.demo.sync.signed")}
          </span>
        </div>
        <div className="ds__row ds__row--server js-server">
          <span>
            <small>{t("web.demo.sync.server")}</small>
            <span className="ds__cp js-server-cp">{cp} #1790</span>
          </span>
          <span className="ds__chip is-bad js-server-chip">
            <Icon name="warning" />
            {t("web.demo.sync.older")}
          </span>
        </div>
      </div>
      <span className="ds__btn js-sync-btn" aria-hidden="true">
        {t("web.demo.sync.now")}
      </span>
      <div className="warn-bar ds__warn js-warn" aria-hidden="true">
        <Icon name="warning" />
        <span>{t("web.how.warning")}</span>
      </div>
      <span className="ds__btn js-checked" aria-hidden="true">
        {t("web.demo.sync.checked")}
      </span>
    </div>
  );
}
