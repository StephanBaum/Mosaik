/**
 * The building blocks of the pages: sections with a soft glow, heads, buttons, product frames
 * and close-up crops (the product is the imagery; there are no illustrations), icon tiles, the
 * capture card and the question list. Shapes snap to an 8 px grid in the stylesheet.
 */
import { Fragment, type ReactNode } from "react";
import { Icon, type IconName } from "./Icon.tsx";
import { Shot, type ShotSpec } from "./Shot.tsx";
import type { Ctx, Translate, WebKey } from "./i18n.ts";
import type { RouteId, Tint } from "./routes.ts";
import { site } from "./site.ts";

const cx = (...names: (string | false | undefined)[]): string => names.filter(Boolean).join(" ");

/** A private-use placeholder for the command while a message is split around it. */
const MARK = "";

/** A message with `{cmd}` rendered as `<code>mosaic</code>`. */
export function Rich({ t, k }: { t: Translate; k: WebKey }) {
  const [before = "", after = ""] = t(k, { cmd: MARK }).split(MARK);
  return (
    <>
      {before}
      <code>mosaic</code>
      {after}
    </>
  );
}

/** A message whose `{ref}` argument renders as `<code>`, for a fixed literal reference. */
export function withCode(t: Translate, key: WebKey, ref: string): ReactNode {
  const [before = "", after = ""] = t(key, { ref: MARK }).split(MARK);
  return (
    <>
      {before}
      <code>{ref}</code>
      {after}
    </>
  );
}

/** A title as one span per sentence, so lines break between sentences, not inside them. */
export function Sentences({ text }: { text: string }) {
  const parts = text.split(/(?<=[.!?])\s+/);
  return (
    <>
      {parts.map((part, index) => (
        <Fragment key={part}>
          {index > 0 ? " " : null}
          <span className="sentence">{part}</span>
        </Fragment>
      ))}
    </>
  );
}

export function Section({
  id,
  tint,
  className,
  children,
  label,
  tabIndex,
}: {
  id: string;
  /** -1 makes the section a jump target that can take focus. */
  tabIndex?: number;
  tint?: Tint;
  className?: string;
  children: ReactNode;
  /** Defaults to the section's `<id>-title` heading. */
  label?: string;
}) {
  return (
    <section
      id={id}
      tabIndex={tabIndex}
      className={cx("section", className)}
      data-tint={tint}
      aria-labelledby={label === undefined ? `${id}-title` : undefined}
      aria-label={label}
    >
      {children}
    </section>
  );
}

export function Eyebrow({ children, className, intro }: { children: ReactNode; className?: string; intro?: string }) {
  return (
    <p className={cx("eyebrow", className)} data-intro={intro}>
      {children}
    </p>
  );
}

export function Head({
  id,
  eyebrow,
  title,
  intro,
  className,
  level = 2,
}: {
  id: string;
  eyebrow?: string;
  title: string;
  intro?: ReactNode;
  className?: string;
  level?: 1 | 2;
}) {
  const Title = level === 1 ? "h1" : "h2";
  return (
    <header className={cx("head", className)} data-reveal="">
      {eyebrow === undefined ? null : <Eyebrow>{eyebrow}</Eyebrow>}
      <Title id={`${id}-title`} className={level === 1 ? "display" : "h2"}>
        <Sentences text={title} />
      </Title>
      {intro === undefined ? null : <p className="lead">{intro}</p>}
    </header>
  );
}

/** The holo foil primary action (ADR-0014): once per viewport, the page's primary action only. */
export function HoloButton({
  href,
  label,
  icon = "download",
  ambient = false,
  jump = false,
}: {
  href: string;
  label: string;
  icon?: IconName;
  /** The foil drifts on its own while on screen (hero only). */
  ambient?: boolean;
  /** An in-page jump: the link scrolls with the dissolve and moves focus to its target. */
  jump?: boolean;
}) {
  return (
    <a className="button button--lg holo" href={href} data-island="holo" data-ambient={ambient ? "true" : undefined} data-jump={jump ? "" : undefined}>
      <canvas className="holo__foil" aria-hidden="true" />
      <span className="holo__label">
        {label}
        <Icon name={icon} />
      </span>
    </a>
  );
}

/** A quiet secondary action: hairline pill with an arrow that nudges on hover. */
export function LineButton({ href, label, large = false, external = false }: { href: string; label: string; large?: boolean; external?: boolean }) {
  return (
    <a className={large ? "button button--lg button--line" : "button button--line"} href={href} rel={external ? "noopener noreferrer" : undefined}>
      {label}
      <Icon name="arrowRight" className="icon--nudge" />
    </a>
  );
}

/** A text link with an arrow, in the section's tint. */
export function ArrowLink({ href, label, className, external = false }: { href: string; label: string; className?: string; external?: boolean }) {
  return (
    <a className={cx("arrow-link", className)} href={href} rel={external ? "noopener noreferrer" : undefined}>
      <span>{label}</span>
      <Icon name="arrowRight" className="icon--nudge" />
    </a>
  );
}

/**
 * The product as the picture: an app screenshot in a precise frame (1 px rim, a top highlight,
 * a soft holo glow behind it). `tilt` is for the one hero shot that may use perspective.
 */
export function ProductFrame({
  ctx,
  shot,
  alt,
  className,
  eager = false,
  tilt = false,
  glow = true,
  showcase = false,
}: {
  ctx: Ctx;
  shot: ShotSpec;
  alt: string;
  className?: string;
  eager?: boolean;
  tilt?: boolean;
  glow?: boolean;
  /** The hero window: it tilts in as the page scrolls (CSS scroll-driven animation). */
  showcase?: boolean;
}) {
  return (
    <div className={cx("frame-wrap", glow && "frame-wrap--glow", tilt && "frame-wrap--tilt", className)} data-showcase={showcase ? "" : undefined}>
      <div className="frame">
        <Shot spec={shot} locale={ctx.locale} alt={alt} eager={eager} />
      </div>
    </div>
  );
}

/**
 * A close-up of one UI detail (a row, a badge, a code ring), cut from a screenshot. The crop
 * window for each `crop` name lives in styles/crops.css, the one place that knows where the
 * detail sits in the shot.
 */
export function Crop({ ctx, shot, crop, alt, className }: { ctx: Ctx; shot: ShotSpec; crop: string; alt: string; className?: string }) {
  return (
    <div className={cx("crop", className)} data-crop={crop}>
      <Shot spec={shot} locale={ctx.locale} alt={alt} />
    </div>
  );
}

/** A small line icon in a rimmed tile: the site's only decoration besides product shots. */
export function IconTile({ name, className }: { name: IconName; className?: string }) {
  return (
    <span className={cx("icon-tile", className)} aria-hidden="true">
      <Icon name={name} />
    </span>
  );
}

/** A numbered card: mono number, title and text. */
export function StepCard({
  n,
  title,
  children,
  className,
}: {
  n: string;
  title?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <li className={cx("step-card", className)}>
      <span className="step-card__n" aria-hidden="true">
        {n}
      </span>
      {title === undefined ? null : <h3 className="step-card__title">{title}</h3>}
      <div className="step-card__text">{children}</div>
    </li>
  );
}

export function Checks({ items, className }: { items: readonly ReactNode[]; className?: string }) {
  return (
    <ul className={cx("checks", className)}>
      {items.map((item, i) => (
        <li key={i}>
          <Icon name="check" className="checks__icon" />
          <span>{item}</span>
        </li>
      ))}
    </ul>
  );
}

/** Questions and answers as native `<details>`, the first one open. */
export function Questions({ t, items, open = 0 }: { t: Translate; items: readonly (readonly [WebKey, WebKey])[]; open?: number }) {
  return (
    <div className="qa-list">
      {items.map(([q, a], index) => (
        <details key={q} className="qa" open={index === open}>
          <summary className="qa__q">
            <h3 className="qa__title">{t(q)}</h3>
            <Icon name="plus" className="qa__icon" />
          </summary>
          <p className="qa__a">{t(a)}</p>
        </details>
      ))}
    </div>
  );
}

/** Breadcrumbs on second-level pages (the JSON-LD twin is written by the page head). */
export function Crumbs({ ctx, trail, intro }: { ctx: Ctx; trail: readonly (readonly [RouteId, string])[]; intro?: string }) {
  return (
    <nav className="crumbs" aria-label={ctx.t("web.crumbs.label")} data-intro={intro}>
      <ol>
        {trail.map(([id, label], index) => (
          <li key={id}>
            {index === trail.length - 1 ? <span aria-current="page">{label}</span> : <a href={ctx.href(id)}>{label}</a>}
          </li>
        ))}
      </ol>
    </nav>
  );
}

export type CaptureList = "switch-kit" | "teams";

/**
 * The capture card (website v2 §2.2, §5.6) with the envelope chip. While `site.captureEnabled`
 * is false it is the honest coming-soon card; with the flag on the same card holds the native
 * form that posts to our own endpoint, with no other change.
 */
export function CaptureCard({ ctx, src, list = "switch-kit", title }: { ctx: Ctx; src: RouteId; list?: CaptureList; title?: string }) {
  const { t, locale } = ctx;
  const id = `capture-${src}`;
  return (
    <aside className="capture" aria-labelledby={`${id}-title`}>
      <IconTile name="mail" className="capture__icon" />
      {site.captureEnabled ? (
        <>
          <h2 id={`${id}-title`} className="capture__title">
            {title ?? t("web.capture.title")}
          </h2>
          <p className="capture__text">{t("web.capture.text")}</p>
          <form className="capture__form" method="post" action="/api/v1/subscribe">
            <label className="capture__label" htmlFor={`${id}-email`}>
              {t("web.capture.label")}
            </label>
            <div className="capture__row">
              <input
                className="input"
                id={`${id}-email`}
                name="email"
                type="email"
                autoComplete="email"
                required
                maxLength={254}
              />
              <button className="button button--solid" type="submit">
                {t("web.capture.button")}
              </button>
            </div>
            <input type="hidden" name="list" value={list} />
            <input type="hidden" name="locale" value={locale} />
            <input type="hidden" name="src" value={src} />
            <input type="hidden" name="consent_v" value="2026-11-08" />
            <input className="capture__trap" type="text" name="website" tabIndex={-1} autoComplete="off" aria-hidden="true" />
            <p className="capture__privacy">{t("web.capture.privacy")}</p>
          </form>
        </>
      ) : (
        <>
          <h2 id={`${id}-title`} className="capture__title">
            {t("web.capture.soon.title")}
          </h2>
          <p className="capture__text">{t("web.capture.soon.text")}</p>
          {src === "switch" ? null : <ArrowLink href={ctx.href("switch")} label={t("web.capture.soon.link")} />}
        </>
      )}
    </aside>
  );
}

/** A feature card: icon, title and text, with an optional visual. `wide` puts the copy beside the visual. */
export function Tile({
  icon,
  title,
  text,
  visual,
  wide = false,
  visualClassName,
}: {
  icon: IconName;
  title: string;
  text: ReactNode;
  visual?: ReactNode;
  wide?: boolean;
  visualClassName?: string;
}) {
  const copy = (
    <>
      <IconTile name={icon} />
      <h3 className="tile__title">{title}</h3>
      <p className="tile__text">{text}</p>
    </>
  );
  if (wide) {
    return (
      <article className="tile tile--wide" data-reveal="">
        <div className="tile__copy">{copy}</div>
        {visual === undefined ? null : <div className={cx("tile__visual", visualClassName)}>{visual}</div>}
      </article>
    );
  }
  return (
    <article className="tile" data-reveal="">
      {visual === undefined ? null : <div className={cx("tile__visual", visualClassName)}>{visual}</div>}
      {copy}
    </article>
  );
}

/** A short point: icon tile beside a bold title and a line of text. */
export function Point({ icon, title, text }: { icon: IconName; title: string; text: ReactNode }) {
  return (
    <div className="point">
      <IconTile name={icon} />
      <div>
        <p className="point__title">{title}</p>
        <p className="point__text">{text}</p>
      </div>
    </div>
  );
}

/** Platform or capability badges: a line icon and a name. */
export function Badges({ ctx, items, center = false, small = false }: { ctx: Ctx; items: readonly (readonly [IconName, WebKey])[]; center?: boolean; small?: boolean }) {
  return (
    <ul className={cx("badges", center && "badges--center", small && "badges--small")}>
      {items.map(([icon, key]) => (
        <li key={key}>
          <Icon name={icon} />
          <span>{ctx.t(key)}</span>
        </li>
      ))}
    </ul>
  );
}
