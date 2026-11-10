/**
 * The password history of an item (M7b): every earlier value, newest first, with its reason. A value
 * from a change that was never confirmed is marked "Not confirmed" (the site may have applied it),
 * and every entry has "Restore", the rollback: the entry becomes the password and the current
 * password goes to the history. Nothing is deleted.
 */
import { useRef } from "react";
import { Button } from "../../components/Button.tsx";
import { SecretField } from "../../components/SecretField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { HistoryEntryView, ItemRequest } from "../../ipc/contract/index.ts";

export function PasswordHistory({
  item,
  history,
  canRestore,
  busy,
  onRestore,
}: {
  item: ItemRequest;
  history: readonly HistoryEntryView[];
  /** Not in the trash, and this account may write. */
  canRestore: boolean;
  busy: boolean;
  /** Resolves when the restore is done (its focus then goes to the heading). */
  onRestore: (entry: HistoryEntryView) => Promise<void>;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const heading = useRef<HTMLHeadingElement>(null);
  if (history.length === 0) {
    return null;
  }
  const hasUnconfirmed = history.some((entry) => entry.reason === "unconfirmed");
  return (
    <section className="detail__notes" aria-label={t("credentialRotation.history.title")}>
      <h3 className="row__label" ref={heading} tabIndex={-1}>
        {t("credentialRotation.history.title")}
      </h3>
      {hasUnconfirmed && <p className="caption">{t("credentialRotation.history.unconfirmedHint")}</p>}
      <ul className="rows" aria-label={t("credentialRotation.history.list")}>
        {history.map((entry) => {
          const unconfirmed = entry.reason === "unconfirmed";
          const label = unconfirmed
            ? t("credentialRotation.history.unconfirmedEntry", { date: entry.changedAt })
            : t("credentialRotation.history.entry", { date: entry.changedAt });
          return (
            <li key={entry.historyId} className="history__entry">
              <SecretField
                label={label}
                reveal={() => ipc.call("reveal_secret", { ...item, secret: { kind: "history", index: entry.index } })}
                actions={
                  unconfirmed ? <span className="chip chip--warning">{t("credentialRotation.history.unconfirmed")}</span> : null
                }
              />
              {canRestore && (
                <Button
                  size="sm"
                  aria-disabled={busy}
                  aria-label={t("credentialRotation.history.restoreFor", { date: entry.changedAt })}
                  onClick={() => {
                    // `aria-disabled`, not `disabled`: the button keeps focus while the restore runs.
                    if (!busy) {
                      void onRestore(entry).then(() => {
                        heading.current?.focus();
                      });
                    }
                  }}
                >
                  {t("credentialRotation.history.restore")}
                </Button>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
