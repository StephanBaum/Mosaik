/**
 * The product half of the page: every-day features with the platform strip, the three
 * things only Mosaic does, and switching from another app.
 */
import { Icon, type IconName } from "./Icon.tsx";
import type { Translate, WebKey } from "./i18n.ts";
import { Head, LineButton, Point, Rich, Section } from "./Section.tsx";

const FEATURES: readonly (readonly [IconName, WebKey, WebKey])[] = [
  ["cursor", "web.features.autofill.title", "web.features.autofill.text"],
  ["key", "web.features.passkeys.title", "web.features.passkeys.text"],
  ["timer", "web.features.codes.title", "web.features.codes.text"],
  ["monitor", "web.features.desktop.title", "web.features.desktop.text"],
  ["phone", "web.features.phone.title", "web.features.phone.text"],
  ["layers", "web.features.more.title", "web.features.more.text"],
  ["spark", "web.features.generator.title", "web.features.generator.text"],
  ["users", "web.features.sharing.title", "web.features.sharing.text"],
  ["buoy", "web.features.emergency.title", "web.features.emergency.text"],
  ["lock", "web.features.ssh.title", "web.features.ssh.text"],
  ["terminal", "web.features.terminal.title", "web.features.terminal.text"],
  ["eyeOff", "web.features.tracking.title", "web.features.tracking.text"],
];

/** Platforms in the copy's order, shared by the feature strip, the download list and the footer. */
export const PLATFORM_KEYS = [
  ["windows", "web.platform.windows", "monitor"],
  ["macos", "web.platform.macos", "monitor"],
  ["linux", "web.platform.linux", "monitor"],
  ["iphone", "web.platform.iphone", "phone"],
  ["android", "web.platform.android", "phone"],
  ["chrome", "web.platform.chrome", "globe"],
  ["edge", "web.platform.edge", "globe"],
  ["firefox", "web.platform.firefox", "globe"],
  ["cli", "web.platform.cli", "terminal"],
] as const satisfies readonly (readonly [string, WebKey, IconName])[];

export function Features({ t }: { t: Translate }) {
  return (
    <Section id="features">
      <Head
        id="features"
        eyebrow={t("web.features.eyebrow")}
        title={t("web.features.title")}
        intro={t("web.features.intro")}
        className="head--wide"
      />
      <ul className="points points--3">
        {FEATURES.map(([icon, title, text]) => (
          <Point
            key={title}
            icon={icon}
            title={t(title)}
            text={text === "web.features.terminal.text" ? <Rich t={t} k={text} /> : t(text)}
          />
        ))}
      </ul>
      <ul className="strip" aria-label={t("web.features.platformsLabel")}>
        {PLATFORM_KEYS.map(([id, key]) => (
          <li key={id}>{t(key)}</li>
        ))}
      </ul>
    </Section>
  );
}

const ONLY = [
  ["web.only.cloud.title", "web.only.cloud.text"],
  ["web.only.notice.title", "web.only.notice.text"],
  ["web.only.guess.title", "web.only.guess.text"],
] as const satisfies readonly (readonly [WebKey, WebKey])[];

const UNLIKE = ["web.only.unlike1", "web.only.unlike2", "web.only.unlike3"] as const satisfies readonly WebKey[];

export function OnlyMosaic({ t }: { t: Translate }) {
  return (
    <Section id="only-mosaic" className="only">
      <Head id="only-mosaic" eyebrow={t("web.only.eyebrow")} title={t("web.only.title")} intro={t("web.only.intro")} />
      <ol className="only__list">
        {ONLY.map(([title, text], index) => (
          <li key={title} className="only__item">
            <p className="only__index" aria-hidden="true">
              {String(index + 1)}
            </p>
            <h3 className="only__title">{t(title)}</h3>
            <p className="only__text">{t(text)}</p>
          </li>
        ))}
      </ol>
      <div className="unlike">
        <h3 className="unlike__title">{t("web.only.unlikeTitle")}</h3>
        <ul className="checks">
          {UNLIKE.map((key) => (
            <li key={key}>
              <Icon name="check" className="checks__icon" />
              <span>{t(key)}</span>
            </li>
          ))}
        </ul>
      </div>
      <aside className="technical" aria-label={t("web.how.technicalLabel")}>
        <p className="technical__label">{t("web.how.technicalLabel")}</p>
        <p className="technical__text">{t("web.only.technical")}</p>
      </aside>
    </Section>
  );
}

const SWITCH: readonly (readonly [IconName, WebKey, WebKey])[] = [
  ["import", "web.switch.bring.title", "web.switch.bring.text"],
  ["key", "web.switch.passkeys.title", "web.switch.passkeys.text"],
  ["export", "web.switch.leave.title", "web.switch.leave.text"],
];

export function Switch({ t }: { t: Translate }) {
  return (
    <Section id="switch">
      <div className="split">
        <div>
          <Head id="switch" eyebrow={t("web.switch.eyebrow")} title={t("web.switch.title")} />
          <p className="body-lg">{t("web.switch.body")}</p>
          <div className="actions">
            <LineButton href="#download" label={t("web.switch.cta")} large />
          </div>
        </div>
        <ul className="inline-points split__aside">
          {SWITCH.map(([icon, title, text]) => (
            <li key={title} className="inline-point">
              <Icon name={icon} className="inline-point__icon" />
              <p>
                <strong>{t(title)}</strong> {t(text)}
              </p>
            </li>
          ))}
        </ul>
      </div>
    </Section>
  );
}
