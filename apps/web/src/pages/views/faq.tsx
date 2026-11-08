/**
 * FAQ (website v2 §5.11): the ten questions in three groups, each led by an icon tile,
 * the first question of each group open. FAQPage JSON-LD goes into the head.
 */
import type { IconName } from "../Icon.tsx";
import type { Ctx, WebKey } from "../i18n.ts";
import { IconTile, Questions, Section } from "../ui.tsx";
import { PageCta, PageHero } from "./parts.tsx";

type Pair = readonly [WebKey, WebKey];

const GROUPS: readonly (readonly [id: string, title: WebKey, icon: IconName, items: readonly Pair[]])[] = [
  ["safety", "web.faqPage.safety", "lock", [["web.faq.q1", "web.faq.a1"], ["web.faq.q3", "web.faq.a3"], ["web.faq.q4", "web.faq.a4"], ["web.faq.q9", "web.faq.a9"]]],
  ["account", "web.faqPage.account", "key", [["web.faq.q2", "web.faq.a2"], ["web.faq.q5", "web.faq.a5"], ["web.faq.q6", "web.faq.a6"]]],
  ["using", "web.faqPage.using", "phone", [["web.faq.q7", "web.faq.a7"], ["web.faq.q8", "web.faq.a8"], ["web.faq.q10", "web.faq.a10"]]],
];

export function Faq({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <>
      <PageHero ctx={ctx} eyebrow={t("web.faq.eyebrow")} title={t("web.faq.title")} />
      {GROUPS.map(([id, title, icon, items]) => (
        <Section key={id} id={id} className="faq-group">
          <div className="wrap faq-group__grid">
            <header className="faq-group__head">
              <IconTile name={icon} />
              <h2 id={`${id}-title`} className="h2">
                {t(title)}
              </h2>
            </header>
            <Questions t={t} items={items} />
          </div>
        </Section>
      ))}
      <PageCta ctx={ctx} title={t("web.download.title")} text={t("web.download.body")} />
    </>
  );
}

export function faqJsonLd(ctx: Ctx): unknown {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: GROUPS.flatMap(([, , , items]) =>
      items.map(([q, a]) => ({
        "@type": "Question",
        name: ctx.t(q),
        acceptedAnswer: { "@type": "Answer", text: ctx.t(a) },
      })),
    ),
  };
}
