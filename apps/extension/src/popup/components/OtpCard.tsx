/**
 * E07: the next step after a password fill, a one-time code. The code is never shown here: it
 * goes from the desktop straight into the field, or into the clipboard on the desktop side.
 */
import type { PopupState } from "../../shared/messages.ts";
import { useI18n } from "../i18n.tsx";
import { Button } from "./Button.tsx";
import { Icon } from "./Icon.tsx";

export function OtpCard({
  next,
  fillOff,
  onFill,
  onCopy,
}: {
  next: NonNullable<PopupState["nextOtp"]>;
  fillOff: boolean;
  onFill: () => void;
  onCopy: () => void;
}) {
  const { t } = useI18n();
  const name = next.username ?? next.title ?? t("ext.item.untitled");
  return (
    <section className="card" aria-labelledby="otp-title">
      <p className="muted" role="status">
        {t("ext.otp.next", { name })}
      </p>
      <h2 id="otp-title" className="title">
        {t("ext.otp.title")}
      </h2>
      <p className="card__row">
        <Icon name="timer" />
        <span>{t("ext.otp.for", { name })}</span>
      </p>
      <div className="actions">
        <Button variant="primary" disabled={fillOff} onClick={onFill}>
          {t("ext.otp.fill")}
        </Button>
        <Button onClick={onCopy}>{t("ext.otp.copy")}</Button>
      </div>
      <p className="caption">{t("ext.otp.note")}</p>
    </section>
  );
}
