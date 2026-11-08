/** Parts shared by the inner pages: the page hero and the closing call to action. */
import type { ReactNode } from "react";
import type { Ctx } from "../i18n.ts";
import type { RouteId } from "../routes.ts";
import { Crumbs, Eyebrow, HoloButton, LineButton, Section, Sentences } from "../ui.tsx";

/**
 * An inner page's hero: calm type on the left (or centred without a visual), and the page's
 * product visual below it (a real screenshot, a crop, or an HTML mockup from the app's tokens).
 */
export function PageHero({
  ctx,
  eyebrow,
  title,
  sub,
  actions,
  visual,
  crumbs,
  className,
}: {
  ctx: Ctx;
  eyebrow?: string;
  title: string;
  sub?: ReactNode;
  actions?: ReactNode;
  visual?: ReactNode;
  crumbs?: readonly (readonly [RouteId, string])[];
  className?: string;
}) {
  return (
    <section className={["hero", "hero--page", visual === undefined ? "hero--plain" : "", className].filter(Boolean).join(" ")} aria-labelledby="hero-title">
      <div className="wrap hero__top">
        {crumbs === undefined ? null : <Crumbs ctx={ctx} trail={crumbs} />}
        {eyebrow === undefined ? null : <Eyebrow>{eyebrow}</Eyebrow>}
        <h1 id="hero-title" className="display">
          <Sentences text={title} />
        </h1>
        {sub === undefined && actions === undefined ? null : (
          <div className="hero__aside">
            {sub === undefined ? null : <p className="lead">{sub}</p>}
            {actions === undefined ? null : <div className="actions">{actions}</div>}
          </div>
        )}
      </div>
      {visual === undefined ? null : <div className="wrap hero__visual">{visual}</div>}
    </section>
  );
}

/** The closing call to action of an inner page: headline, one holo button, one line button. */
export function PageCta({ ctx, title, text, secondary }: { ctx: Ctx; title: string; text?: string; secondary?: readonly [href: string, label: string] }) {
  return (
    <Section id="get" className="page-cta">
      <div className="wrap page-cta__inner">
        <h2 id="get-title" className="h2">
          <Sentences text={title} />
        </h2>
        {text === undefined ? null : <p className="lead">{text}</p>}
        <div className="actions actions--center">
          <HoloButton href={ctx.href("download")} label={ctx.t("web.download.ctaPrimary")} />
          {secondary === undefined ? null : <LineButton href={secondary[0]} label={secondary[1]} large />}
        </div>
      </div>
    </Section>
  );
}
