/**
 * E04: a login waits to be saved. Title and vault are chosen in the Mosaic app, where the
 * confirmation happens, so this card only shows the site, the user name and a hidden password
 * (ADR-0038). "Save in Mosaic…" asks the desktop, which opens its own dialog.
 */
import { useState } from "react";
import { errorCodeOf } from "../../shared/errors.ts";
import type { PopupState } from "../../shared/messages.ts";
import { useI18n } from "../i18n.tsx";
import { useServices } from "../usePopup.ts";
import { useAnnounce } from "./Announcer.tsx";
import { Avatar, Button } from "./Button.tsx";

type Pending = NonNullable<PopupState["pendingSave"]>;

export function SaveCard({ pending }: { pending: Pending }) {
  const { t, errorText } = useI18n();
  const { api, refresh } = useServices();
  const announce = useAnnounce();
  const [waiting, setWaiting] = useState(false);
  const host = pending.unicodeHost ?? pending.host;

  async function answer(action: "save" | "dismiss" | "never") {
    setWaiting(action === "save");
    try {
      const { outcome } = await api.request({ op: "savePending", action });
      announce(outcome === "never" ? t("ext.save.neverDone", { host }) : t(`ext.save.${outcome}`));
    } catch (error) {
      announce(errorText(errorCodeOf(error)));
    } finally {
      setWaiting(false);
      await refresh();
    }
  }

  return (
    <section className="card" aria-labelledby="save-title">
      <div className="card__head">
        <Avatar text={host} />
        <h2 id="save-title" className="title">
          {t(pending.kind === "update" ? "ext.save.titleUpdate" : "ext.save.titleNew", { host })}
        </h2>
      </div>
      {pending.source === "generated" && <p className="muted">{t("ext.save.generated", { host })}</p>}
      {pending.kind === "update" && pending.itemTitle !== null && (
        <p className="muted">{t("ext.save.updates", { title: pending.itemTitle })}</p>
      )}
      <dl className="facts">
        <dt>{t("ext.save.username")}</dt>
        <dd>{pending.username ?? t("ext.save.noUsername")}</dd>
        <dt>{t("ext.save.password")}</dt>
        <dd>
          <span aria-hidden="true">••••••••</span>
          <span className="visually-hidden">{t("ext.save.passwordHidden")}</span>
        </dd>
      </dl>
      {pending.insecure && <p className="caption">{t("ext.save.insecure")}</p>}
      {waiting && (
        <p className="caption" role="status">
          {t("ext.save.waiting")}
        </p>
      )}
      <div className="actions">
        <Button variant="primary" disabled={waiting} onClick={() => void answer("save")}>
          {t("ext.save.action")}
        </Button>
        <Button disabled={waiting} onClick={() => void answer("dismiss")}>
          {t("ext.save.notNow")}
        </Button>
        <Button variant="link" disabled={waiting} onClick={() => void answer("never")}>
          {t("ext.save.never")}
        </Button>
      </div>
    </section>
  );
}
