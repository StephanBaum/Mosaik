/**
 * Pricing (two equal cards, Hosted with the accent rim) and teams and family.
 */
import { Icon, type IconName } from "./Icon.tsx";
import type { Translate, WebKey } from "./i18n.ts";
import { Head, LineButton, Point, Rich, Section } from "./Section.tsx";

interface Plan {
  id: "free" | "hosted";
  name: WebKey;
  price: WebKey;
  tagline: WebKey;
  items: readonly WebKey[];
  cta: WebKey;
}

const PLANS: readonly Plan[] = [
  {
    id: "free",
    name: "web.pricing.free.name",
    price: "web.pricing.free.price",
    tagline: "web.pricing.free.tagline",
    items: [
      "web.pricing.free.item1",
      "web.pricing.free.item2",
      "web.pricing.free.item3",
      "web.pricing.free.item4",
      "web.pricing.free.item5",
      "web.pricing.free.item6",
      "web.pricing.free.item7",
    ],
    cta: "web.pricing.free.cta",
  },
  {
    id: "hosted",
    name: "web.pricing.hosted.name",
    price: "web.pricing.hosted.price",
    tagline: "web.pricing.hosted.tagline",
    items: [
      "web.pricing.hosted.item1",
      "web.pricing.hosted.item2",
      "web.pricing.hosted.item3",
      "web.pricing.hosted.item4",
      "web.pricing.hosted.item5",
      "web.pricing.hosted.item6",
    ],
    cta: "web.pricing.hosted.cta",
  },
];

function PlanCard({ t, plan }: { t: Translate; plan: Plan }) {
  return (
    <li className={`plan plan--${plan.id}`}>
      <h3 className="plan__name">{t(plan.name)}</h3>
      <p className={plan.id === "free" ? "plan__price" : "plan__price plan__price--soon"}>{t(plan.price)}</p>
      <p className="plan__tagline">{t(plan.tagline)}</p>
      <ul className="checks plan__items">
        {plan.items.map((key) => (
          <li key={key}>
            <Icon name="check" className="checks__icon" />
            <span>{key === "web.pricing.free.item7" ? <Rich t={t} k={key} /> : t(key)}</span>
          </li>
        ))}
      </ul>
      <div className="plan__action">
        <LineButton href="#download" label={t(plan.cta)} />
      </div>
    </li>
  );
}

export function Pricing({ t }: { t: Translate }) {
  return (
    <Section id="pricing" className="pricing">
      <Head id="pricing" eyebrow={t("web.pricing.eyebrow")} title={t("web.pricing.title")} intro={t("web.pricing.intro")} />
      <ul className="plans">
        {PLANS.map((plan) => (
          <PlanCard key={plan.id} t={t} plan={plan} />
        ))}
      </ul>
      <p className="fineprint pricing__note">{t("web.pricing.note")}</p>
    </Section>
  );
}

const TEAMS: readonly (readonly [IconName, WebKey, WebKey])[] = [
  ["vault", "web.teams.shared.title", "web.teams.shared.text"],
  ["shield", "web.teams.verify.title", "web.teams.verify.text"],
  ["userMinus", "web.teams.goodbye.title", "web.teams.goodbye.text"],
  ["cloud", "web.teams.company.title", "web.teams.company.text"],
  ["buoy", "web.teams.emergency.title", "web.teams.emergency.text"],
];

export function Teams({ t }: { t: Translate }) {
  return (
    <Section id="teams" className="teams">
      <div className="split split--top">
        <div className="teams__lead">
          <Head id="teams" eyebrow={t("web.teams.eyebrow")} title={t("web.teams.title")} />
          <p className="body-lg">{t("web.teams.body")}</p>
          <div className="actions">
            <LineButton href="#download" label={t("web.teams.cta")} large />
          </div>
        </div>
        <ul className="points points--stack split__aside">
          {TEAMS.map(([icon, title, text]) => (
            <Point key={title} icon={icon} title={t(title)} text={t(text)} />
          ))}
        </ul>
      </div>
    </Section>
  );
}
