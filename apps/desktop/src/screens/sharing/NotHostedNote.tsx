/** Says why sharing is not available: no sync yet, or a sync folder (which cannot share). */
import type { MessageKey } from "@mosaic/i18n";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { BackendKind } from "./hooks.ts";

export function NotHostedNote({
  kind,
  message,
  onOpenSync,
}: {
  kind: Exclude<BackendKind, "hosted" | "loading">;
  /** Another text than the one for sharing (emergency access). */
  message?: MessageKey;
  /** Shows a button to the sync settings. */
  onOpenSync?: () => void;
}) {
  const { t } = useI18n();
  return (
    <div className="stack">
      <Callout tone="info">
        <p className="body">{t(message ?? (kind === "folder" ? "sharing.notHosted.folder" : "sharing.notHosted.none"))}</p>
      </Callout>
      {onOpenSync !== undefined && (
        <div className="actions">
          <Button size="sm" onClick={onOpenSync}>
            {t("sharing.syncSettings")}
          </Button>
        </div>
      )}
    </div>
  );
}
