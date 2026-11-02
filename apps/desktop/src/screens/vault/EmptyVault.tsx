/** D09: the empty vault, centered across the whole main area. */
import { Button } from "../../components/Button.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { HoloSurface } from "../../components/HoloSurface.tsx";
import { Icon } from "../../components/icons.tsx";
import { Kbd } from "../../components/Kbd.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { usePlatform } from "../../platform/platform.ts";
import { ariaKeyshortcuts, SHORTCUTS } from "../../platform/shortcuts.ts";

function Illustration() {
  return (
    <span className="illu-wrap">
      <HoloSurface className="illu__foil" />
      <svg className="illu" width="120" height="120" viewBox="0 0 120 120" aria-hidden="true" focusable="false">
        <rect className="illu__tile" x="8" y="8" width="48" height="48" rx="12" />
        <rect className="illu__tile" x="64" y="8" width="48" height="48" rx="12" />
        <rect className="illu__tile illu__tile--dashed" x="8" y="64" width="48" height="48" rx="12" />
        <rect className="illu__tile illu__tile--solid" x="64" y="64" width="48" height="48" rx="12" />
        <path className="illu__plus" d="M88 79v18M79 88h18" />
      </svg>
    </span>
  );
}

export function EmptyVault({ onNew, onImport }: { onNew: () => void; onImport: () => void }) {
  const { t } = useI18n();
  const platform = usePlatform();
  return (
    <div className="empty-state">
      <Illustration />
      <div className="empty-state__text">
        <h2 className="display">{t("vault.emptyTitle")}</h2>
        <p className="lede empty-state__lede">{t("vault.emptyBody")}</p>
      </div>
      <div className="actions empty-state__actions">
        <HoloButton
          ambient
          onClick={onNew}
          aria-keyshortcuts={ariaKeyshortcuts(SHORTCUTS.newItem, platform)}
        >
          <Icon name="plus" />
          {t("vault.addFirst")}
          <Kbd shortcut="newItem" hidden />
        </HoloButton>
        <Button onClick={onImport}>
          <Icon name="download" />
          {t("palette.cmd.import")}
        </Button>
      </div>
      <p className="caption">
        {t("vault.paletteTip.before")} <Kbd shortcut="palette" /> {t("vault.paletteTip.after")}
      </p>
    </div>
  );
}
