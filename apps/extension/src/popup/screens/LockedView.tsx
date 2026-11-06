/** E01 without the master password: unlocking happens only in the Mosaic app (ADR-0035). */
import type { PopupState } from "../../shared/messages.ts";
import { Button } from "../components/Button.tsx";
import { Logo } from "../components/Shell.tsx";
import { useI18n } from "../i18n.tsx";
import { useActions } from "../useActions.ts";

export function LockedView({ state }: { state: PopupState }) {
  const { t } = useI18n();
  const { openDesktop } = useActions();
  const host = state.tab?.host;
  return (
    <section className="center" aria-labelledby="locked-title">
      <Logo size={36} />
      <h1 id="locked-title" className="title">
        {t("ext.locked.title")}
      </h1>
      <p className="muted">{host === undefined ? t("ext.locked.bodyNoSite") : t("ext.locked.body", { host })}</p>
      <Button variant="primary" block onClick={() => void openDesktop()}>
        {t("ext.locked.open")}
      </Button>
      <p className="caption">{t("ext.locked.note")}</p>
    </section>
  );
}
