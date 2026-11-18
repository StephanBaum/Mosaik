/**
 * The five next steps (prototype 4507): numbered cards under a line that draws itself, each card
 * getting a tick. Rendered finished (line drawn, ticks shown); `src/demos/nextSteps.ts` replays
 * the build-up once, when the list scrolls into view.
 */
import { Icon } from "../Icon.tsx";
import type { Ctx, WebKey } from "../i18n.ts";
import type { RouteId } from "../routes.ts";
import { ArrowLink } from "../ui.tsx";

const STEPS: readonly { key: WebKey; link?: readonly [RouteId, WebKey] }[] = [
  { key: "web.next.s1" },
  { key: "web.next.s2" },
  { key: "web.next.s3", link: ["switch", "web.next.s3link"] },
  { key: "web.next.s4" },
  { key: "web.next.s5", link: ["pricing", "web.next.s5link"] },
];

export function NextSteps({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <ol className="steps" data-island="demos" data-demo="nextSteps">
      <svg className="steps__line" viewBox="0 0 1000 28" preserveAspectRatio="none" aria-hidden="true">
        <path className="js-steps-path" d="M0 14 H1000" vectorEffect="non-scaling-stroke" />
      </svg>
      {STEPS.map(({ key, link }, i) => {
        const [lead = "", ...rest] = t(key).split(/(?<=\.)\s/);
        return (
          <li key={key} className="step-card">
            <span className="step-card__n" aria-hidden="true">
              {`0${String(i + 1)}`}
            </span>
            <span className="step-card__tick js-tick" aria-hidden="true">
              <Icon name="check" />
            </span>
            <h3 className="step-card__title">{lead}</h3>
            <div className="step-card__text">
              <p>{rest.join(" ")}</p>
              {link === undefined ? null : <ArrowLink href={ctx.href(link[0])} label={t(link[1])} />}
            </div>
          </li>
        );
      })}
    </ol>
  );
}
