/** The terminal: three commands are typed and answered in turn (prototype 4836). The runs are fixed literals from docs/CLI.md. */
import type { Timeline } from "../anim/timeline.ts";
import { after, pick, story, typeTo, wait } from "./text.ts";

/** Command, text before the emphasised result, the emphasised result. Mirrors `CLI_RUNS` in the markup. */
const RUNS = [
  ["mosaic totp github", "", "492 039"],
  ["mosaic run --env GITHUB_TOKEN=mosaic://work/github/token -- gh repo list", "12 repositories · token ", "masked"],
  ["mosaic ssh list", "ed25519 deploy@example.com · ", "asks before every use"],
] as const;

export function build(root: HTMLElement): Timeline | null {
  const cmd = pick(root, ".js-cmd");
  const out = pick(root, ".js-out");
  const doc = root.ownerDocument;
  const show = (before: string, strong: string): void => {
    const bold = doc.createElement("strong");
    bold.textContent = strong;
    out.replaceChildren(...(before === "" ? [bold] : [before, bold]));
  };
  const tl = story(1.5);
  RUNS.forEach(([command, before, strong], i) => {
    tl.call(() => {
      cmd.textContent = "";
      out.replaceChildren();
    });
    typeTo(tl, cmd, command, { duration: 1.1, at: i === 0 ? 0.4 : after(0.3) });
    tl.call(() => {
      show(before, strong);
    }, "+=0.4")
      .set(out, { opacity: 0, y: 4 })
      .to(out, { opacity: 1, y: 0 }, { duration: 0.3 });
    wait(tl, i === RUNS.length - 1 ? 1.5 : 2.2);
  });
  return tl;
}
