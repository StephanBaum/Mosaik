/**
 * Exact-origin fill in a browser window (prototype 4179). Rendered in its final frame, the
 * look-alike address: the address bar flags it, the popup offers nothing and the status says so.
 * `src/demos/fill.ts` replays the story from the data attributes; no copy lives in script.
 */
import { Icon } from "../Icon.tsx";
import type { Ctx } from "../i18n.ts";

/** The look-alike: a digit one for the first "l" (login.yourbank.com becomes 1ogin.yourbank.com). */
export function lookalike(site: string): string {
  return site.includes("l") ? site.replace("l", "1") : `${site.split(".")[0] ?? site}1.${site.split(".").slice(1).join(".")}`;
}

interface Words {
  site: string;
  user: string;
  bad: string;
  t: Ctx["t"];
}

/** The address bar on the look-alike: the first digit one is set apart, and the tag flags it. */
function Chrome({ bad, t }: Words) {
  const at = bad.indexOf("1");
  return (
    <div className="db__chrome" aria-hidden="true">
      <span className="db__dots">
        <i />
        <i />
        <i />
      </span>
      <span className="db__url js-url is-bad">
        <Icon name="lock" />
        <span className="js-url-text">
          {bad.slice(0, at)}
          <span className="bad">1</span>
          {bad.slice(at + 1)}
        </span>
        <span className="db__tag js-tag">{t("web.demo.fill.lookalike")}</span>
      </span>
    </div>
  );
}

/** The fill popup: the matching login offers itself, the look-alike offers nothing (its row shows). */
function Popup({ site, user, bad, t }: Words) {
  return (
    <div className="mock db__popup js-popup is-no">
      <div className="fill">
        <div className="fill__bar">
          <Icon name="lock" />
          <span className="js-popup-site">{bad}</span>
        </div>
        <div className="fill__row fill__row--ok">
          <span className="fill__avatar">A</span>
          <span className="fill__who">
            <strong>{site}</strong>
            <span>{user}</span>
          </span>
          <span className="fill__go js-go">{t("web.featuresPage.fill.action")}</span>
        </div>
        <div className="fill__row fill__row--no">
          <Icon name="shieldOff" />
          <span className="fill__who">
            <strong>{t("web.demo.fill.nomatch")}</strong>
            <span>{t("web.demo.fill.nothing")}</span>
          </span>
        </div>
      </div>
    </div>
  );
}

function Form(words: Words) {
  const { t } = words;
  return (
    <div className="db__form">
      <h4>{t("web.demo.fill.signin")}</h4>
      <div className="db__field">
        <label>{t("web.demo.fill.email")}</label>
        <div className="db__input js-email is-focus">
          <span className="js-email-text" />
        </div>
        <Popup {...words} />
      </div>
      <div className="db__field">
        <label>{t("web.demo.fill.password")}</label>
        <div className="db__input js-pass">
          <span className="dots is-empty js-dots">
            {Array.from({ length: 12 }, (_, i) => (
              <i key={i} />
            ))}
          </span>
        </div>
      </div>
      <div className="db__submit">{t("web.demo.fill.continue")}</div>
    </div>
  );
}

export function FillDemo({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const site = t("web.featuresPage.fill.site");
  const user = t("web.featuresPage.fill.user");
  const words: Words = { site, user, bad: lookalike(site), t };
  return (
    <div
      className="db"
      data-island="demos"
      data-demo="fill"
      data-site={site}
      data-bad={words.bad}
      data-user={user}
      data-text-ok={t("web.demo.fill.statusOk", { site })}
      data-text-no={t("web.demo.fill.statusNo")}
      role="img"
      aria-label={t("web.featuresPage.fill.label")}
    >
      <Chrome {...words} />
      <div className="db__page js-page" aria-hidden="true">
        <Form {...words} />
        <svg className="db__cursor js-cursor" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <path d="M5 3l14 8-6 1.5L10 19z" className="db__cursor-edge" strokeWidth="1.2" strokeLinejoin="round" />
        </svg>
        <span className="db__status js-status is-no">
          <Icon name="shield" />
          <span className="js-status-text">{t("web.demo.fill.statusNo")}</span>
        </span>
      </div>
    </div>
  );
}
