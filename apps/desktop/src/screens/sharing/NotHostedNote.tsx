/** Says why sharing is not available: no sync yet, or a sync folder (which cannot share). */
import { Callout } from "../../components/Callout.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { BackendKind } from "./hooks.ts";

export function NotHostedNote({ kind }: { kind: Exclude<BackendKind, "hosted" | "loading"> }) {
  const { t } = useI18n();
  return (
    <Callout tone="info">
      <p className="body">{t(kind === "folder" ? "sharing.notHosted.folder" : "sharing.notHosted.none")}</p>
    </Callout>
  );
}
