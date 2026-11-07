/**
 * Shared building blocks of the page sections: the section frame, its head (eyebrow, h2,
 * intro), the holo primary action and text with the `mosaic` command set as code.
 */
import { Fragment, type ReactNode } from "react";
import { Icon, type IconName } from "./Icon.tsx";
import type { Translate, WebKey } from "./i18n.ts";

/** A private-use placeholder for the command while a message is split around it. */
const MARK = "";

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

/**
 * A title as one span per sentence. Each span wraps as a unit where it fits, so lines break
 * between sentences ("Everything you log in with. / Handled.") and not inside them.
 */
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
  className,
  children,
}: {
  id: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section id={id} className={["section", className].filter(Boolean).join(" ")} aria-labelledby={`${id}-title`}>
      <div className="section__inner">{children}</div>
    </section>
  );
}

export function Head({
  id,
  eyebrow,
  title,
  intro,
  className,
}: {
  id: string;
  eyebrow: string;
  title: string;
  intro?: string;
  className?: string;
}) {
  return (
    <header className={["head", className].filter(Boolean).join(" ")}>
      <p className="eyebrow">{eyebrow}</p>
      <h2 id={`${id}-title`} className="head__title">
        <Sentences text={title} />
      </h2>
      {intro === undefined ? null : <p className="head__intro">{intro}</p>}
    </header>
  );
}

/** The holo foil primary action (ADR-0014). The foil canvas fades in once the stage paints it. */
export function HoloButton({ href, label, icon = "download" }: { href: string; label: string; icon?: IconName }) {
  return (
    <a className="button button--lg holo" href={href}>
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

/** An icon, a bold lead-in and its text: the item of every feature-like list on the page. */
export function Point({ icon, title, text }: { icon: IconName; title: string; text: ReactNode }) {
  return (
    <li className="point">
      <Icon name={icon} className="point__icon" />
      <h3 className="point__title">{title}</h3>
      <p className="point__text">{text}</p>
    </li>
  );
}
