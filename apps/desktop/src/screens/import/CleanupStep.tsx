/** The last step: delete the plaintext export (honestly), and what to switch off in the old app. */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { ImportResult, ImportSyncedFolder, IpcError } from "../../ipc/contract/index.ts";
import { CopyAddress } from "./SourceSteps.tsx";
import type { ImportSourceInfo } from "./sources.ts";

type Deletion = "idle" | "busy" | "deleted" | "failed";

/**
 * True when everything came across: only then is the file safe to remove. `clean` is the
 * shell's verdict (nothing skipped, failed or left out of an entry); it is the authority, the
 * counts are kept as a second guard.
 */
export function importComplete(result: ImportResult): boolean {
  return result.clean && result.failed === 0 && result.attachmentsFailed === 0;
}

function DeleteFile({ token, fileName, complete }: { token: string; fileName: string; complete: boolean }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [state, setState] = useState<Deletion>("idle");
  const [error, setError] = useState<IpcError | null>(null);
  async function remove() {
    setState("busy");
    const result = await ipc.call("import_delete_file", { fileToken: token });
    setState(result.ok ? "deleted" : "failed");
    setError(result.ok ? null : result.error);
  }
  if (!complete) {
    return (
      <Callout tone="warning">
        <UserText>{t("import.cleanup.keepFile", { name: fileName })}</UserText>
      </Callout>
    );
  }
  return (
    <div className="stack">
      {state === "deleted" ? (
        <p className="body row__status" role="status">
          <Icon name="check" />
          <UserText>{t("import.cleanup.deleted", { name: fileName })}</UserText>
        </p>
      ) : (
        <HoloButton disabled={state === "busy"} onClick={() => void remove()}>
          {t("import.cleanup.delete", { name: fileName })}
        </HoloButton>
      )}
      {state === "failed" && (
        <>
          <p className="alert" role="alert">
            <UserText>{t("import.cleanup.deleteFailed", { name: fileName })}</UserText>
          </p>
          <ErrorAlert error={error} />
        </>
      )}
    </div>
  );
}

export function CleanupStep({
  source,
  token,
  fileName,
  synced,
  result,
  onDone,
}: {
  source: ImportSourceInfo;
  token: string;
  fileName: string;
  synced: ImportSyncedFolder | null;
  result: ImportResult;
  onDone: () => void;
}) {
  const { t } = useI18n();
  const name = t(`import.source.${source.id}`);
  return (
    <div className="stack">
      <p className="body" role="status">
        {t("import.result", { imported: result.imported, skipped: result.skipped })}{" "}
        {t("import.flow.resultFailed", { failed: result.failed })}
      </p>
      <h2 className="subtitle">{t("import.cleanup.title")}</h2>
      <DeleteFile token={token} fileName={fileName} complete={importComplete(result)} />
      <p className="field__hint">{t("import.cleanup.ssdNote")}</p>
      {synced !== null && (
        <Callout tone="warning">
          {t("import.cleanup.synced", { service: t(`import.synced.${synced}`) })}
        </Callout>
      )}
      <h2 className="subtitle">{t("import.checklist.title")}</h2>
      <ul className="guide-checklist">
        {source.checklist.map((item) => (
          <li key={item}>
            {t(`import.checklist.${item}`, { source: name })}
            {item === "browserSaving" && source.internalUrl !== null && (
              <CopyAddress address={source.internalUrl} />
            )}
          </li>
        ))}
      </ul>
      <Button variant="secondary" onClick={onDone}>
        {t("action.done")}
      </Button>
    </div>
  );
}
