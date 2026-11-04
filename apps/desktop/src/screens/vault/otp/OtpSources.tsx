/** The ways to add a one-time code: scan the screen, paste a screenshot, pick an image, type the key. */
import { Button } from "../../../components/Button.tsx";
import { Icon } from "../../../components/icons.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import { usePlatform } from "../../../platform/platform.ts";
import type { OtpStart } from "./OtpSection.tsx";

export function OtpSources({
  screenScan,
  onPick,
}: {
  /** Whether this system can scan the screen (`app_status.otpScreenScan`). */
  screenScan: boolean;
  onPick: (start: OtpStart) => void;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  return (
    <div className="otp-sources">
      <div className="otp-sources__buttons" role="group" aria-label={t("otp.section.label")}>
        {screenScan && (
          <Button
            onClick={() => {
              onPick({ kind: "scan", source: "screen" });
            }}
          >
            <Icon name="scan" />
            {t("otp.source.screen")}
          </Button>
        )}
        <Button
          onClick={() => {
            onPick({ kind: "scan", source: "clipboard" });
          }}
        >
          <Icon name="clipboard" />
          {t("otp.source.clipboard")}
        </Button>
        <Button
          onClick={() => {
            onPick({ kind: "scan", source: "file" });
          }}
        >
          <Icon name="upload" />
          {t("otp.source.file")}
        </Button>
      </div>
      {screenScan && <p className="field__hint">{t("otp.source.screenHint")}</p>}
      <p className="field__hint">{t("otp.source.clipboardHint", { platform })}</p>
      <div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            onPick({ kind: "manual" });
          }}
        >
          {t("otp.source.manual")}
        </Button>
      </div>
    </div>
  );
}
