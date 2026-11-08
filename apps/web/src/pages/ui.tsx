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
}: {
  id: string;
  tint?: Tint;
  className?: string;
  children: ReactNode;
  /** Defaults to the section's `<id>-title` heading. */
  label?: string;
}) {
  return (
    <section
      id={id}
      className={cx("section", className)}
      data-tint={tint}
      aria-labelledby={label === undefined ? `${id}-title` : undefined}
      aria-label={label}
    >
      {children}
    </section>
  );
}

export function Eyebrow({ children, className }: { children: ReactNode; className?: string }) {
  return <p className={cx("eyebrow", className)}>{children}</p>;
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
    <header className={cx("head", className)}>
      {eyebrow === undefined ? null : <Eyebrow>{eyebrow}</Eyebrow>}
      <Title id={`${id}-title`} className={level === 1 ? "display" : "h2"}>
        <Sentences text={title} />
      </Title>
      {intro === undefined ? null : <p className="lead">{intro}</p>}
    </header>
  );
}

/** The holo foil primary action (ADR-0014): once per viewport, the page's primary action only. */
export function HoloButton({ href, label, icon = "download" }: { href: string; label: string; icon?: IconName }) {
  return (
    <a className="button button--lg holo" href={href} data-island="holo">
      <canvas className="holo__foil" aria-hidden="true" />
      <span className="holo__label">
        {label}
        <Icon name={icon} />
      </span>
    </a>
  );
}

/** A quiet secondary action: hairline pill with an arrow that nudges on hover. */
export function LineButton({ href, label, large = false }: { href: string; label: string; large?: boolean }) {
  return (
    <a className={large ? "button button--lg button--line" : "button button--line"} href={href}>
      {label}
      <Icon name="arrowRight" className="icon--nudge" />
    </a>
  );
}

/** A text link with an arrow, in the section's tint. */
export function ArrowLink({ href, label, className }: { href: string; label: string; className?: string }) {
  return (
    <a className={cx("arrow-link", className)} href={href}>
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
}: {
  ctx: Ctx;
  shot: ShotSpec;
  alt: string;
  className?: string;
  eager?: boolean;
  tilt?: boolean;
  glow?: boolean;
}) {
  return (
    <div className={cx("frame-wrap", glow && "frame-wrap--glow", tilt && "frame-wrap--tilt", className)}>
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
export function Crumbs({ ctx, trail }: { ctx: Ctx; trail: readonly (readonly [RouteId, string])[] }) {
  return (
    <nav className="crumbs" aria-label={ctx.t("web.crumbs.label")}>
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
