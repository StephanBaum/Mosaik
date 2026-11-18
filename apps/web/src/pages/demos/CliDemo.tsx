/**
 * The terminal demo (prototype 4256). The commands are real syntax from docs/CLI.md with fixed
 * example values; the demo types three runs and rests on the last. `src/demos/cli.ts` replays it.
 */
import type { Ctx } from "../i18n.ts";
import { Rich } from "../ui.tsx";

/** The runs the demo types: command, text before the emphasised result, the emphasised result. */
export const CLI_RUNS = [
  ["mosaic totp github", "", "492 039"],
  ["mosaic run --env GITHUB_TOKEN=mosaic://work/github/token -- gh repo list", "12 repositories · token ", "masked"],
  ["mosaic ssh list", "ed25519 deploy@example.com · ", "asks before every use"],
] as const;

export function CliDemo({ ctx, withTitle = false }: { ctx: Ctx; withTitle?: boolean }) {
  const { t } = ctx;
  const [cmd, before, strong] = CLI_RUNS[CLI_RUNS.length - 1] ?? CLI_RUNS[0];
  return (
    <div className="terminal" data-island="demos" data-demo="cli" role="group" aria-label={t("web.features.terminal.title")}>
      <div className="terminal__bar" aria-hidden="true">
        <i />
        <i />
        <i />
      </div>
      <div className="terminal__body">
        {withTitle ? (
          <>
            <h3 className="terminal__title">{t("web.features.terminal.title")}</h3>
            <p className="terminal__text"><Rich t={t} k="web.features.terminal.text" /></p>
          </>
        ) : null}
        <p className="terminal__line" aria-hidden="true">
          <span className="terminal__prompt">$</span> <span className="js-cmd">{cmd}</span>
          <span className="terminal__caret" />
        </p>
        <p className="terminal__out js-out" aria-hidden="true">
          {before}
          <strong>{strong}</strong>
        </p>
      </div>
    </div>
  );
}
