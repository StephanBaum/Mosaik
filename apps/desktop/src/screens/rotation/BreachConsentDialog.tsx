/**
 * The consent for the breach check (M7b, OD-M7b-3, R97): the check is off until the user agrees
 * here, and the text says plainly what Have I Been Pwned learns. "Not now" changes nothing.
 */
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";

export function BreachConsentDialog({ onAgree, onDecline }: { onAgree: () => void; onDecline: () => void }) {
  const { t } = useI18n();
  return (
    <Dialog
      title={t("breach.consent.title")}
      onClose={onDecline}
      icon={<Icon name="shieldAlert" />}
      returnFocus={() => document.querySelector<HTMLElement>("[data-breach-card]")}
    >
      <p>{t("breach.consent.body")}</p>
      <p className="caption">{t("breach.consent.detail")}</p>
      <div className="dialog__actions">
        {/* Declining is the default: the first control Tab reaches and the one that has focus. */}
        <Button onClick={onDecline} data-autofocus="">
          {t("breach.consent.decline")}
        </Button>
        <HoloButton onClick={onAgree}>{t("breach.consent.agree")}</HoloButton>
      </div>
    </Dialog>
  );
}
