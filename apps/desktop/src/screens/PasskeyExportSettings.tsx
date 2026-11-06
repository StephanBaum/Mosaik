/**
 * "Export passkeys" (M5, ADR-0041): saves the ES256 passkeys of one vault as a FIDO CXF file to
 * move them to another app. The file holds the private keys unencrypted, so the panel says so
 * and the export asks for the master password. Rust opens the save dialog and writes the file;
 * this panel only sends the vault and the password.
 */
import { useEffect, useState } from "react";
import { Button } from "../components/Button.tsx";
import { ReauthDialog } from "../components/ReauthDialog.tsx";
import { Select } from "../components/Select.tsx";
import { useToast } from "../components/ToastProvider.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { VaultId } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { SettingsSection } from "./SettingsSection.tsx";
import { useWritableVaults } from "./useWritableVaults.ts";

/** How many passkeys the vault holds (`null` while it is being counted). */
function usePasskeyCount(vaultId: VaultId | null): number | null {
  const ipc = useIpc();
  const [counted, setCounted] = useState<{ vaultId: VaultId; count: number } | null>(null);
  useEffect(() => {
    if (vaultId === null) {
      return;
    }
    let current = true;
    void ipc
      .call("list_items", { vaultId, trash: "exclude", favoritesOnly: false, query: null })
      .then((result) => {
        if (current) {
          const count = result.ok ? result.value.filter((item) => item.kind === "passkey").length : 0;
          setCounted({ vaultId, count });
        }
      });
    return () => {
      current = false;
    };
  }, [ipc, vaultId]);
  return counted !== null && counted.vaultId === vaultId ? counted.count : null;
}

export function PasskeyExportSettings() {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const vaults = useWritableVaults();
  const [chosen, setChosen] = useState<VaultId | null>(null);
  const [ask, setAsk] = useState(false);
  const vaultId = chosen ?? vaults[0]?.vaultId ?? null;
  const count = usePasskeyCount(vaultId);

  if (vaults.length === 0) {
    return null;
  }
  return (
    <SettingsSection title={t("settings.passkeyExport")} body={t("settings.passkeyExport.body")}>
      <Select
        inline
        label={t("settings.passkeyExport.vault")}
        value={vaultId ?? ""}
        options={vaults.map((vault) => ({ value: vault.vaultId, label: vault.name ?? t("vault.unnamed") }))}
        onChange={(event) => {
          setChosen(vaults.find((vault) => vault.vaultId === event.target.value)?.vaultId ?? null);
        }}
      />
      <p className="field__hint" role="note">
        {t("settings.passkeyExport.warning")}
      </p>
      {count === 0 && <p className="field__hint">{t("settings.passkeyExport.none")}</p>}
      <div className="setting-row">
        <Button
          disabled={vaultId === null || count === null || count === 0}
          onClick={() => {
            setAsk(true);
          }}
        >
          {t("settings.passkeyExport.button")}
        </Button>
      </div>
      {ask && vaultId !== null && (
        <ReauthDialog
          body="settings.passkeyExport.reauth"
          onSubmit={async (password) => {
            const result = await ipc.call("export_passkeys_cxf", { vaultId, password });
            if (!result.ok) {
              return result.error;
            }
            if (result.value.kind === "saved") {
              toast(t("settings.passkeyExport.saved", { count: result.value.count }));
            }
            return null;
          }}
          onClose={() => {
            setAsk(false);
          }}
        />
      )}
    </SettingsSection>
  );
}
