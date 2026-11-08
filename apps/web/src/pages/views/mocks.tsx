/**
 * Small UI mockups built in HTML and CSS from the app's design tokens, for screens the app
 * shows but a screenshot cannot isolate (the fill popup on a site, the sync warning, a members
 * list, platform badges, what a stolen server holds). They are labelled as samples for
 * assistive tech and never stand in for an illustration.
 */
import type { Ctx, WebKey } from "../i18n.ts";
import { Icon, type IconName } from "../Icon.tsx";

/** The fill popup on the site a login belongs to. */
export function FillMock({ ctx, className }: { ctx: Ctx; className?: string }) {
  const { t } = ctx;
  return (
    <div className={["mock", "mock--fill", className].filter(Boolean).join(" ")} role="img" aria-label={t("web.featuresPage.fill.label")}>
      <div className="fill" aria-hidden="true">
        <div className="fill__bar">
          <Icon name="lock" />
          <span>{t("web.featuresPage.fill.site")}</span>
        </div>
        <div className="fill__row">
          <span className="fill__avatar">A</span>
          <span className="fill__who">
            <strong>{t("web.featuresPage.fill.site")}</strong>
            <span>{t("web.featuresPage.fill.user")}</span>
          </span>
          <span className="fill__go">{t("web.featuresPage.fill.action")}</span>
        </div>
        <div className="fill__dots">
          {Array.from({ length: 12 }, (_, i) => (
            <i key={i} />
          ))}
        </div>
      </div>
    </div>
  );
}

/** The warning {name} keeps up when a synced copy is older than one the devices have seen. */
export function WarnMock({ ctx, className }: { ctx: Ctx; className?: string }) {
  const { t } = ctx;
  return (
    <div className={["mock", "mock--warn", className].filter(Boolean).join(" ")} role="img" aria-label={t("web.how.warningAlt")}>
      <span className="mock__tag" aria-hidden="true">
        {t("web.how.sample")}
      </span>
      <span className="warn-bar" aria-hidden="true">
        <Icon name="warning" />
        <span>{t("web.how.warning")}</span>
      </span>
    </div>
  );
}

/** A shared vault's members with their safety-code state. */
export function MembersMock({ ctx, className }: { ctx: Ctx; className?: string }) {
  const { t } = ctx;
  const people = [
    ["A", "Anna", "anna@example.com", true],
    ["J", "Jonas", "jonas@example.com", true],
    ["M", "Mia", "mia@example.com", false],
  ] as const;
  return (
    <div className={["mock", "mock--members", className].filter(Boolean).join(" ")} role="img" aria-label={t("web.teamsPage.mock.label")}>
      <div className="members" aria-hidden="true">
        <div className="members__head">
          <Icon name="layers" />
          <strong>{t("web.teamsPage.mock.vault")}</strong>
          <span className="members__count">{t("web.teamsPage.mock.count")}</span>
        </div>
        <ul className="members__list">
          {people.map(([initial, name, mail, verified]) => (
            <li key={name}>
              <span className="members__avatar">{initial}</span>
              <span className="members__who">
                <strong>{name}</strong>
                <span>{mail}</span>
              </span>
              <span className={verified ? "badge badge--ok" : "badge"}>
                <Icon name={verified ? "shield" : "timer"} />
                {t(verified ? "web.teamsPage.mock.verified" : "web.teamsPage.mock.pending")}
              </span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

/** Rows of sealed data: what a copy of the server holds. */
export function CipherMock({ ctx, className }: { ctx: Ctx; className?: string }) {
  const rows = [
    "8f3a 91c2 e07b 4d16 aa58 0c9e 7713 b2f4",
    "c41d 0e7a 39b8 f265 1d0c 88a3 5e4f 06b9",
    "27e9 b3f0 6a1c d854 90e2 4b7d c3a6 1f08",
    "e5b2 7c49 02fd a816 3e9b 5d70 f1c4 98a2",
    "3b06 d9e1 47a8 2cf5 b06d 81e3 6a2b c7f9",
  ];
  return (
    <div className={["mock", "mock--cipher", className].filter(Boolean).join(" ")} role="img" aria-label={ctx.t("web.security.stolen.mock")}>
      <div className="cipher" aria-hidden="true">
        {rows.map((row, i) => (
          <code key={row}>
            <span className="cipher__id">item_{String(i + 1).padStart(2, "0")}</span>
            {row}
          </code>
        ))}
      </div>
    </div>
  );
}

/** Platform badges: line icon plus name. */
export function PlatformBadges({ ctx, items, className }: { ctx: Ctx; items: readonly (readonly [IconName, WebKey])[]; className?: string }) {
  return (
    <ul className={["badges", className].filter(Boolean).join(" ")}>
      {items.map(([icon, key]) => (
        <li key={key}>
          <Icon name={icon} />
          <span>{ctx.t(key)}</span>
        </li>
      ))}
    </ul>
  );
}

export const ALL_PLATFORMS = [
  ["monitor", "web.platform.windows"],
  ["monitor", "web.platform.macos"],
  ["monitor", "web.platform.linux"],
  ["phone", "web.platform.iphone"],
  ["phone", "web.platform.android"],
  ["globe", "web.platform.chrome"],
  ["globe", "web.platform.edge"],
  ["globe", "web.platform.firefox"],
  ["terminal", "web.platform.cli"],
] as const satisfies readonly (readonly [IconName, WebKey])[];
