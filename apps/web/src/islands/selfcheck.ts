/**
 * The self-check island: scores the ticked answers and reveals the matching result and next
 * steps, all in the page. It reads only radio buttons, stores nothing (no storage, no URL
 * state) and sends nothing. Without JS the questions stay readable and the page says the
 * result needs JavaScript.
 */
import { levelFor, TIPS, type TipId } from "../pages/checkData.ts";

const MAX_TIPS = 3;

/** The result for a set of ticked answers: the level and up to three next steps, in priority order. */
export function evaluate(answers: readonly { points: number; tip?: string }[]): { level: string; tips: TipId[] } {
  const score = answers.reduce((sum, a) => sum + a.points, 0);
  const wanted = new Set(answers.map((a) => a.tip).filter((tip): tip is string => tip !== undefined));
  const tips = TIPS.filter((tip) => wanted.has(tip)).slice(0, MAX_TIPS);
  return { level: levelFor(score), tips: tips.length > 0 ? tips : ["keep"] };
}

export function bootSelfCheck(root: Document): void {
  for (const box of root.querySelectorAll<HTMLElement>("[data-island='selfcheck']")) {
    const questions = [...box.querySelectorAll<HTMLFieldSetElement>("fieldset")];
    const go = box.querySelector<HTMLButtonElement>(".check__go");
    const hint = box.querySelector<HTMLElement>(".check__hint");
    const result = box.querySelector<HTMLElement>(".check__result");
    const again = box.querySelector<HTMLButtonElement>(".check__again");
    if (go === null || result === null) {
      continue;
    }
    box.classList.add("is-live");
    const checked = (): HTMLInputElement[] =>
      questions.flatMap((q) => {
        const input = q.querySelector<HTMLInputElement>("input:checked");
        return input === null ? [] : [input];
      });
    const update = (): void => {
      const done = checked().length === questions.length;
      go.disabled = !done;
      if (hint !== null) {
        hint.hidden = done;
      }
    };
    box.addEventListener("change", update);
    go.addEventListener("click", () => {
      const { level, tips } = evaluate(
        checked().map((input) => {
          const tip = input.dataset["tip"];
          return tip === undefined ? { points: Number(input.dataset["points"] ?? 0) } : { points: Number(input.dataset["points"] ?? 0), tip };
        }),
      );
      for (const el of result.querySelectorAll<HTMLElement>("[data-level]")) {
        el.hidden = el.dataset["level"] !== level;
      }
      for (const el of result.querySelectorAll<HTMLElement>("li[data-tip]")) {
        el.hidden = !tips.includes(el.dataset["tip"] as TipId);
      }
      // Keep the tips in priority order.
      const list = result.querySelector(".check__tips");
      for (const tip of tips) {
        const li = list?.querySelector(`li[data-tip="${tip}"]`);
        if (li !== null && li !== undefined) {
          list?.append(li);
        }
      }
      result.hidden = false;
      result.focus();
      result.scrollIntoView({ block: "start", behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
    });
    again?.addEventListener("click", () => {
      for (const input of box.querySelectorAll<HTMLInputElement>("input[type='radio']")) {
        input.checked = false;
      }
      result.hidden = true;
      update();
      questions[0]?.querySelector("input")?.focus();
    });
    update();
  }
}
