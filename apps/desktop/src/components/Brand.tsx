/** Logo mark and product name. */
import { useI18n } from "../i18n/I18nProvider.tsx";
import { LogoMark } from "./LogoMark.tsx";

export function Brand() {
  const { t } = useI18n();
  return (
    <span className="brand">
      <LogoMark />
      <span className="brand__name">{t("app.name")}</span>
    </span>
  );
}
