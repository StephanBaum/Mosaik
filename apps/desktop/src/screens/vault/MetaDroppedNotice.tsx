/**
 * "Vault details were shortened" (CRYPTO_SPEC section 7.1): a merge of a vault's name, icon or
 * description was too long to keep whole and the longer value won. The notice names the fields
 * in plain words (never the values) and stays until the user dismisses it. It is a polite status
 * region; dismissing moves focus to the page when the button that had it is gone.
 */
import { useState } from "react";
import { useEvents } from "../../app/EventsProvider.tsx";
import type { MetaDropped } from "../../app/useNoticeEvents.ts";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { VaultSummary } from "../../ipc/contract/index.ts";
import { focusPageIfLost } from "../focusPage.ts";
import { EditVaultDialog } from "./EditVaultDialog.tsx";
import { vaultLabel } from "./Sidebar.tsx";

function Entry({
  entry,
  vaults,
  onChanged,
}: {
  entry: MetaDropped;
  vaults: readonly VaultSummary[];
  onChanged: () => void;
}) {
  const { t, locale } = useI18n();
  const [editing, setEditing] = useState(false);
  const { notices } = useEvents();
  const vault = vaults.find((v) => v.vaultId === entry.vaultId);
  const fields = new Intl.ListFormat(locale, { style: "long", type: "conjunction" }).format(
    entry.fields.map((field) => t(`vault.meta.field.${field}`)),
  );
  return (
    <Callout tone="info">
      <p className="body">
        <strong>{t("vault.meta.title")}</strong>
      </p>
      <p className="body">
        {t("vault.meta.body", {
          fields,
          vault: vault === undefined ? t("vault.unnamed") : vaultLabel(vault, (key) => t(key)),
        })}
      </p>
      <div className="actions">
        {vault !== undefined && (
          <Button
            onClick={() => {
              setEditing(true);
            }}
          >
            {t("vault.meta.edit")}
          </Button>
        )}
        <Button
          variant="ghost"
          onClick={() => {
            notices.dismissMetaDropped(entry.vaultId);
            // The button goes with the notice: focus would fall to the page's top.
            focusPageIfLost();
          }}
        >
          {t("vault.meta.dismiss")}
        </Button>
      </div>
      {editing && vault !== undefined && (
        <EditVaultDialog
          vault={vault}
          onSaved={onChanged}
          onClose={() => {
            setEditing(false);
          }}
        />
      )}
    </Callout>
  );
}

export function MetaDroppedNotice({
  vaults,
  onChanged,
}: {
  vaults: readonly VaultSummary[];
  onChanged: () => void;
}) {
  const { notices } = useEvents();
  // The region stays mounted so that a notice that appears is announced.
  return (
    <div role="status">
      {notices.metaDropped.map((entry) => (
        <Entry key={entry.vaultId} entry={entry} vaults={vaults} onChanged={onChanged} />
      ))}
    </div>
  );
}
