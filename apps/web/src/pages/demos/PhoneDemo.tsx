/**
 * Phone autofill (prototype 4240): the app's sign-in screen, the suggestion above the keyboard
 * and the biometric check. The customer number is a fixed literal. Rendered in its final frame
 * (keyboard down, login filled); `src/demos/phone.ts` replays the story.
 */
import { Icon } from "../Icon.tsx";
import type { Ctx } from "../i18n.ts";
import { LogoMark } from "../Logo.tsx";

export const PHONE_USER = "1049 3312";

export function PhoneDemo({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <div className="dp" data-island="demos" data-demo="phone" role="img" aria-label={t("web.features.phone.text")}>
      <div className="dp__phone" aria-hidden="true">
        <div className="dp__notch" />
        <div className="dp__app">
          <div className="dp__brand">
            <i />
            {t("web.tools.phone.app")}
          </div>
          <p className="dp__title">{t("web.tools.phone.signin")}</p>
          <div className="db__field">
            <label>{t("web.tools.phone.user")}</label>
            <div className="db__input js-puser is-focus">
              <span className="js-puser-text">{PHONE_USER}</span>
            </div>
          </div>
          <div className="db__field">
            <label>{t("web.tools.phone.pass")}</label>
            <div className="db__input js-ppass">
              <span className="dots js-pdots">
                {Array.from({ length: 10 }, (_, i) => (
                  <i key={i} />
                ))}
              </span>
            </div>
          </div>
          <div className="db__submit">{t("web.tools.phone.button")}</div>
        </div>
        <div className="dp__kb is-down js-kb">
          <div className="dp__sugg">
            <span className="dp__chip js-chip">
              <LogoMark />
              <Icon name="key" />
              <span>
                {t("web.tools.phone.app")} · {PHONE_USER}
              </span>
            </span>
          </div>
          <div className="dp__keys">
            {[10, 9, 7, 1].map((n) => (
              <div key={n}>
                {Array.from({ length: n }, (_, i) => (
                  <i key={i} />
                ))}
              </div>
            ))}
          </div>
        </div>
        <span className="dp__face js-face">
          <Icon name="lock" />
          {t("web.tools.phone.unlock")}
        </span>
        <i className="dp__tap js-tap" />
      </div>
    </div>
  );
}
