/** Step 2: the numbered export guide for one source, then the file dialog and the drop hint. */
import type { MessageKey } from "@mosaic/i18n";
import { Button } from "../../components/Button.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { ImportSourceInfo } from "./sources.ts";

/** An address to paste into the product, with a copy button. */
export function CopyAddress({ address }: { address: string }) {
  const { t, errorMessage } = useI18n();
  const toast = useToast();
  async function copy() {
    try {
      await navigator.clipboard.writeText(address);
      toast(t("import.guide.urlCopied"), { tone: "success" });
    } catch {
      toast(errorMessage({ code: "clipboardUnavailable" }));
    }
  }
  return (
    <div className="copy-address">
      <code className="mono copy-address__code">{address}</code>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => {
          void copy();
        }}
      >
        <Icon name="copy" />
        {t("import.guide.copyUrl")}
      </Button>
    </div>
  );
}

export function SourceSteps({
  source,
  busy,
  onChoose,
  onBack,
}: {
  source: ImportSourceInfo;
  busy: boolean;
  onChoose: () => void;
  onBack: () => void;
}) {
  const { t } = useI18n();
  const steps = Array.from({ length: source.steps }, (_, i) => i + 1);
  return (
    <div className="stack">
      <h2 className="subtitle">{t(`import.source.${source.id}`)}</h2>
      <ol className="guide-steps">
        {steps.map((n) => (
          <li key={n}>
            {t(`import.guide.${source.id}.${String(n)}` as MessageKey)}
            {n === 1 && source.internalUrl !== null && <CopyAddress address={source.internalUrl} />}
          </li>
        ))}
      </ol>
      <div className="dropzone">
        <Icon name="upload" size="lg" className="muted" />
        <p className="dropzone__text">{t("import.guide.dropHint")}</p>
        <HoloButton disabled={busy} onClick={onChoose}>
          {t("import.guide.choose")}
        </HoloButton>
      </div>
      <Button variant="ghost" onClick={onBack}>
        {t("import.guide.back")}
      </Button>
    </div>
  );
}
